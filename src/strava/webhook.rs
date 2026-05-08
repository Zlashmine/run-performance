/// Strava webhook handlers.
///
/// Routes:
///   GET  /webhooks/strava — subscription validation challenge
///   POST /webhooks/strava — incoming activity/athlete events
///
/// Signature verification uses HMAC-SHA256 over `"{t}.{raw_body}"` where `t`
/// is the timestamp extracted from the `X-Strava-Signature` header.
/// Format: `t={ts},v1={hex_mac}`
///
/// Replay protection: reject if |now - t| > 300 seconds.
use actix_web::{get, post, web, HttpRequest, HttpResponse};
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use sqlx::PgPool;
use std::collections::HashMap;

use super::client::{find_user_by_athlete_id, delete_tokens, StravaClient};
use crate::{activities::repository, error::AppError};

type HmacSha256 = Hmac<Sha256>;

// ─── GET — subscription validation ───────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
pub struct HubChallenge {
    #[serde(rename = "hub.mode")]
    pub mode:         String,
    #[serde(rename = "hub.challenge")]
    pub challenge:    String,
    #[serde(rename = "hub.verify_token")]
    pub verify_token: String,
}

#[utoipa::path(
    get,
    path = "/webhooks/strava",
    tag = "strava",
    responses((status = 200, description = "Challenge echoed"))
)]
#[get("/webhooks/strava")]
pub async fn validate_webhook(query: web::Query<HubChallenge>) -> Result<HttpResponse, AppError> {
    let expected = std::env::var("STRAVA_WEBHOOK_VERIFY_TOKEN").unwrap_or_default();
    if query.verify_token != expected {
        tracing::warn!("Webhook validate: bad verify_token");
        return Err(AppError::Unauthorized);
    }
    Ok(HttpResponse::Ok().json(serde_json::json!({ "hub.challenge": query.challenge })))
}

// ─── POST — incoming events  ──────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct StravaEvent {
    pub object_type:  String,
    pub object_id:    i64,
    pub aspect_type:  String,
    pub owner_id:     i64,
    pub updates:      Option<HashMap<String, String>>,
}

#[utoipa::path(
    post,
    path = "/webhooks/strava",
    tag = "strava",
    responses((status = 200, description = "Accepted"))
)]
#[post("/webhooks/strava")]
pub async fn receive_event(
    req:    HttpRequest,
    db:     web::Data<PgPool>,
    client: web::Data<StravaClient>,
    body:   web::Bytes,
) -> Result<HttpResponse, AppError> {
    // ── 1. Verify signature ──────────────────────────────────────────────────
    let sig_header = req
        .headers()
        .get("X-Strava-Signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !verify_signature(sig_header, &body) {
        tracing::warn!("Webhook: bad signature");
        return Err(AppError::Unauthorized);
    }

    // ── 2. Parse event ───────────────────────────────────────────────────────
    let event: StravaEvent = serde_json::from_slice(&body).map_err(|e| {
        tracing::warn!("Webhook parse error: {e}");
        AppError::BadRequest("invalid event payload".into())
    })?;

    // ── 3. Respond 200 immediately, dispatch in background ──────────────────
    let db_bg     = db.into_inner();
    let client_bg = client.into_inner();
    tokio::spawn(async move {
        if let Err(e) = process_event(&event, &db_bg, &client_bg).await {
            tracing::error!("webhook process_event failed: {e:?}");
        }
    });

    Ok(HttpResponse::Ok().finish())
}

// ─── Signature verification ──────────────────────────────────────────────────

/// Verifies `X-Strava-Signature: t={ts},v1={hex}`.
/// Signed payload is `"{ts}.{raw_body}"`, key is `STRAVA_WEBHOOK_SIGNING_SECRET`.
/// Rejects timestamps older than 5 minutes (replay protection).
fn verify_signature(header: &str, body: &[u8]) -> bool {
    let secret = match std::env::var("STRAVA_WEBHOOK_SIGNING_SECRET") {
        Ok(s) if !s.is_empty() => s,
        _ => {
            tracing::error!("STRAVA_WEBHOOK_SIGNING_SECRET not set");
            return false;
        }
    };

    // Parse header: "t=1234567890,v1=abc123..."
    let mut ts_str: Option<&str>  = None;
    let mut v1:     Option<&str>  = None;
    for part in header.split(',') {
        if let Some(v) = part.strip_prefix("t=")  { ts_str = Some(v); }
        if let Some(v) = part.strip_prefix("v1=") { v1     = Some(v); }
    }

    let (ts_str, signature_hex) = match (ts_str, v1) {
        (Some(t), Some(v)) => (t, v),
        _ => {
            tracing::warn!("Webhook: malformed signature header");
            return false;
        }
    };

    // Replay check.
    let ts: i64 = match ts_str.parse() {
        Ok(t) => t,
        Err(_) => return false,
    };
    let now = Utc::now().timestamp();
    if (now - ts).abs() > 300 {
        tracing::warn!("Webhook: timestamp out of window ({ts})");
        return false;
    }

    // Compute expected HMAC.
    let signed_payload = format!("{}.{}", ts_str, String::from_utf8_lossy(body));
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m)  => m,
        Err(_) => return false,
    };
    mac.update(signed_payload.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());

    // Constant-time comparison.
    expected.len() == signature_hex.len()
        && expected
            .bytes()
            .zip(signature_hex.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

// ─── Event processing ─────────────────────────────────────────────────────────

async fn process_event(
    event:  &StravaEvent,
    db:     &PgPool,
    client: &StravaClient,
) -> Result<(), AppError> {
    let athlete_id = event.owner_id;

    match (event.object_type.as_str(), event.aspect_type.as_str()) {
        // ── New activity created ─────────────────────────────────────────────
        ("activity", "create") => {
            let user_id = match find_user_by_athlete_id(db, athlete_id).await? {
                Some(id) => id,
                None     => {
                    tracing::warn!("Webhook create: no user for athlete {athlete_id}");
                    return Ok(());
                }
            };

            let token = client.get_valid_token(db, user_id).await?;
            let detail = client.get_activity(&token, event.object_id).await?;
            let streams = client.get_streams(&token, event.object_id).await.unwrap_or(
                super::client::StreamSet { latlng: None, altitude: None, time: None, velocity_smooth: None }
            );

            let start_dt = match chrono::DateTime::parse_from_rfc3339(&detail.start_date) {
                Ok(dt) => dt.with_timezone(&Utc),
                Err(e) => {
                    tracing::warn!("bad start_date in webhook: {e}");
                    return Ok(());
                }
            };

            let normalized = super::client::normalize(&detail, streams, start_dt);
            crate::activities::service::ingest_activities(db, user_id, &[normalized]).await;
        }

        // ── Activity deleted ─────────────────────────────────────────────────
        ("activity", "delete") => {
            let user_id = match find_user_by_athlete_id(db, athlete_id).await? {
                Some(id) => id,
                None     => return Ok(()),
            };
            let external_id = event.object_id.to_string();
            repository::delete_by_external_id(db, user_id, "strava", &external_id).await?;
        }

        // ── Athlete deauthorized ─────────────────────────────────────────────
        ("athlete", "update") => {
            let is_deauth = event
                .updates
                .as_ref()
                .and_then(|u| u.get("authorized"))
                .map(|v| v == "false")
                .unwrap_or(false);

            if is_deauth {
                let user_id = match find_user_by_athlete_id(db, athlete_id).await? {
                    Some(id) => id,
                    None     => return Ok(()),
                };
                delete_tokens(db, user_id).await?;
                tracing::info!("Strava deauthorized for user {user_id}");
            }
        }

        (obj, asp) => {
            tracing::debug!("Webhook: unhandled event {obj}/{asp}");
        }
    }

    Ok(())
}
