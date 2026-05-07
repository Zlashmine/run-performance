/// Strava activity sync handler.
///
/// Route: POST /sync/strava/{user_id}
///
/// Returns 202 immediately and runs the full backfill in a background task.
/// If `since` query param is provided (Unix seconds), only activities after
/// that timestamp are fetched; otherwise fetches all history (since=0).
use actix_web::{post, web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use super::client::{normalize, StravaClient};
use crate::{activities::service::ingest_activities, error::AppError};

// ─── Handler ──────────────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct SyncQuery {
    /// Unix epoch seconds; only sync activities after this timestamp.
    pub since: Option<i64>,
}

#[utoipa::path(
    post,
    path = "/sync/strava/{user_id}",
    tag = "strava",
    responses(
        (status = 202, description = "Sync started"),
        (status = 404, description = "Strava not connected for user"),
    )
)]
#[post("/sync/strava/{user_id}")]
pub async fn sync_handler(
    db:     web::Data<PgPool>,
    client: web::Data<StravaClient>,
    path:   web::Path<Uuid>,
    query:  web::Query<SyncQuery>,
) -> Result<HttpResponse, AppError> {
    let user_id = path.into_inner();
    let since   = query.since.unwrap_or(0);

    // Verify connection exists before accepting the request.
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM strava_tokens WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_one(db.get_ref())
    .await
    .map_err(AppError::from)?;

    if count == 0 {
        return Err(AppError::NotFound);
    }

    let db_bg     = db.into_inner();
    let client_bg = client.into_inner();
    tokio::spawn(async move {
        if let Err(e) = run_sync((*client_bg).clone(), &db_bg, user_id, since).await {
            tracing::error!("strava sync for {user_id} failed: {e:?}");
        }
    });

    Ok(HttpResponse::Accepted().finish())
}

// ─── Core sync logic (also called from auth::connect_handler) ─────────────────

/// Pull all Strava activities for `user_id` that started after `since` (Unix seconds).
pub async fn run_sync(
    client:  StravaClient,
    db:      &PgPool,
    user_id: Uuid,
    since:   i64,
) -> Result<(), AppError> {
    let token = client.get_valid_token(db, user_id).await?;
    let mut page = 1u32;

    loop {
        let summaries = match client.list_activities(&token, since, page).await {
            Ok(v)  => v,
            Err(e) => {
                tracing::error!("list_activities page {page}: {e:?}");
                break;
            }
        };

        if summaries.is_empty() {
            break;
        }

        for summary in &summaries {
            let activity_id = summary.id;

            // Fetch full detail + streams concurrently.
            let (detail_result, stream_result) = tokio::join!(
                client.get_activity(&token, activity_id),
                client.get_streams(&token, activity_id),
            );

            let detail = match detail_result {
                Ok(d)  => d,
                Err(e) => {
                    tracing::warn!("skip activity {activity_id}: {e:?}");
                    continue;
                }
            };

            // Parse the start date — skip activities with unparseable timestamps.
            let start_dt = match chrono::DateTime::parse_from_rfc3339(&detail.start_date) {
                Ok(dt) => dt.with_timezone(&Utc),
                Err(e) => {
                    tracing::warn!("bad start_date for {activity_id}: {e}");
                    continue;
                }
            };

            let streams = stream_result.unwrap_or_else(|e| {
                tracing::warn!("streams for {activity_id} unavailable: {e:?}");
                super::client::StreamSet {
                    latlng: None, altitude: None, time: None, velocity_smooth: None,
                }
            });

            let normalized = normalize(&detail, streams, start_dt);
            ingest_activities(db, user_id, &[normalized]).await;
        }

        page += 1;
    }

    // Record the last sync timestamp.
    let _ = sqlx::query(
        "UPDATE strava_tokens SET last_synced_at = NOW() WHERE user_id = $1"
    )
    .bind(user_id)
    .execute(db)
    .await;

    tracing::info!("strava sync complete for {user_id}");
    Ok(())
}
