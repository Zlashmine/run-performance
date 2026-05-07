/// Strava OAuth handlers.
///
/// Routes:
///   POST /strava/connect          — exchange auth code, store tokens, kick off backfill
///   POST /strava/disconnect/{uid} — revoke + delete tokens
///   GET  /strava/status/{uid}     — return connection status
use actix_web::{delete, get, post, web, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::client::{upsert_tokens, delete_tokens, StravaClient};
use crate::error::AppError;

// ─── Request / response types ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ConnectRequest {
    pub user_id:      Uuid,
    pub code:         String,
    pub redirect_uri: String,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub connected:      bool,
    pub athlete_name:   Option<String>,
    pub last_synced_at: Option<chrono::DateTime<chrono::Utc>>,
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

/// Exchange an authorization code and persist tokens.
#[utoipa::path(
    post,
    path = "/strava/connect",
    tag = "strava",
    responses((status = 204, description = "Connected"))
)]
#[post("/strava/connect")]
pub async fn connect_handler(
    db:     web::Data<PgPool>,
    client: web::Data<StravaClient>,
    body:   web::Json<ConnectRequest>,
) -> Result<HttpResponse, AppError> {
    let tokens = client
        .exchange_code(&body.code, &body.redirect_uri)
        .await?;

    upsert_tokens(&db, body.user_id, &tokens).await?;

    // Kick off a background backfill from the last 90 days.
    let db_bg     = db.into_inner();
    let client_bg = client.into_inner();
    let user_id   = body.user_id;
    tokio::spawn(async move {
        let since = chrono::Utc::now().timestamp() - 90 * 24 * 3600;
        if let Err(e) = super::sync::run_sync((*client_bg).clone(), &db_bg, user_id, since).await {
            tracing::error!("background backfill for {user_id} failed: {e:?}");
        }
    });

    Ok(HttpResponse::NoContent().finish())
}

/// Revoke Strava access and remove tokens from the database.
#[utoipa::path(
    delete,
    path = "/strava/disconnect/{user_id}",
    tag = "strava",
    responses((status = 204, description = "Disconnected"))
)]
#[delete("/strava/disconnect/{user_id}")]
pub async fn disconnect_handler(
    db:      web::Data<PgPool>,
    client:  web::Data<StravaClient>,
    path:    web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let user_id = path.into_inner();

    // Get current access token (best-effort — proceed even if not found).
    if let Ok(token) = client.get_valid_token(&db, user_id).await {
        client.deauthorize(&token).await?;
    }

    delete_tokens(&db, user_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Return Strava connection status for a user.
#[utoipa::path(
    get,
    path = "/strava/status/{user_id}",
    tag = "strava",
    responses((status = 200, description = "Status"))
)]
#[get("/strava/status/{user_id}")]
pub async fn status_handler(
    db:   web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let user_id = path.into_inner();

    #[derive(sqlx::FromRow)]
    struct StatusRow { strava_athlete_name: String, last_synced_at: Option<chrono::DateTime<chrono::Utc>> }
    let row = sqlx::query_as::<_, StatusRow>(
        "SELECT strava_athlete_name, last_synced_at
         FROM strava_tokens WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(db.get_ref())
    .await
    .map_err(AppError::from)?;

    let response = match row {
        None => StatusResponse {
            connected:      false,
            athlete_name:   None,
            last_synced_at: None,
        },
        Some(r) => StatusResponse {
            connected:      true,
            athlete_name:   Some(r.strava_athlete_name),
            last_synced_at: r.last_synced_at,
        },
    };

    Ok(HttpResponse::Ok().json(response))
}
