use actix_web::{web, HttpResponse};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MissionHistoryEntry {
    pub id: Uuid,
    pub mission_type: String,
    pub title: String,
    pub xp_reward: i32,
    pub completed_at: DateTime<Utc>,
    pub is_boss: bool,
    /// "weekly" or "monthly"
    pub cadence: String,
    pub period_start: NaiveDate,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MissionHistoryResponse {
    pub entries: Vec<MissionHistoryEntry>,
    /// Total XP earned from all completed missions (aggregate, not paginated).
    pub total_xp_from_missions: i64,
    /// Cursor for the next page — the `completed_at` of the 51st result, if any.
    pub next_cursor: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryParams {
    /// ISO-8601 timestamp cursor for pagination (exclusive upper bound on completed_at).
    pub cursor: Option<DateTime<Utc>>,
}

/// Row type for the UNION history query.
#[derive(sqlx::FromRow)]
struct HistoryRow {
    id: Uuid,
    mission_type: String,
    title: String,
    xp_reward: i32,
    completed_at: DateTime<Utc>,
    is_boss: bool,
    cadence: String,
    period_start: NaiveDate,
}

/// GET /users/{user_id}/missions/history
///
/// Returns paginated completed missions (weekly + monthly) ordered newest first.
/// Uses cursor-based pagination: pass `?cursor=<ISO timestamp>` for next page.
#[utoipa::path(
    get,
    path = "/users/{user_id}/missions/history",
    params(
        ("user_id" = Uuid, Path, description = "User ID"),
        ("cursor" = Option<String>, Query, description = "Pagination cursor (completed_at timestamp)"),
    ),
    responses(
        (status = 200, description = "Mission history", body = MissionHistoryResponse),
    ),
    tag = "Missions"
)]
pub async fn get_mission_history(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    query: web::Query<HistoryParams>,
) -> Result<HttpResponse, AppError> {
    let user_id = path.into_inner();
    let cursor = query.cursor;

    // ── 1. Aggregate total XP (fast, uses partial indexes) ────────────────────
    let total_xp: i64 = {
        let weekly: Option<i64> = sqlx::query_scalar(
            r#"SELECT COALESCE(SUM(xp_reward), 0)::BIGINT FROM weekly_missions
               WHERE user_id = $1 AND completed_at IS NOT NULL"#,
        )
        .bind(user_id)
        .fetch_one(&**pool)
        .await
        .map_err(|e| { tracing::error!("history total_xp weekly error: {e}"); AppError::Internal })?;

        let monthly: Option<i64> = sqlx::query_scalar(
            r#"SELECT COALESCE(SUM(xp_reward), 0)::BIGINT FROM monthly_missions
               WHERE user_id = $1 AND completed_at IS NOT NULL"#,
        )
        .bind(user_id)
        .fetch_one(&**pool)
        .await
        .map_err(|e| { tracing::error!("history total_xp monthly error: {e}"); AppError::Internal })?;

        weekly.unwrap_or(0) + monthly.unwrap_or(0)
    };

    // ── 2. Paginated UNION query (limit 51 to detect next page) ───────────────
    let rows = sqlx::query_as::<_, HistoryRow>(
        r#"
        SELECT id, mission_type, title, xp_reward, completed_at,
               false AS is_boss, 'weekly' AS cadence, week_start AS period_start
        FROM weekly_missions
        WHERE user_id = $1
          AND completed_at IS NOT NULL
          AND ($2::TIMESTAMPTZ IS NULL OR completed_at < $2)

        UNION ALL

        SELECT id, mission_type, title, xp_reward, completed_at,
               is_boss, 'monthly' AS cadence, month_start AS period_start
        FROM monthly_missions
        WHERE user_id = $1
          AND completed_at IS NOT NULL
          AND ($2::TIMESTAMPTZ IS NULL OR completed_at < $2)

        ORDER BY completed_at DESC
        LIMIT 51
        "#,
    )
    .bind(user_id)
    .bind(cursor)
    .fetch_all(&**pool)
    .await
    .map_err(|e| { tracing::error!("mission history query error: {e}"); AppError::Internal })?;

    let next_cursor = if rows.len() > 50 {
        rows.get(50).map(|r| r.completed_at)
    } else {
        None
    };

    let entries = rows
        .into_iter()
        .take(50)
        .map(|r| MissionHistoryEntry {
            id: r.id,
            mission_type: r.mission_type,
            title: r.title,
            xp_reward: r.xp_reward,
            completed_at: r.completed_at,
            is_boss: r.is_boss,
            cadence: r.cadence,
            period_start: r.period_start,
        })
        .collect();

    Ok(HttpResponse::Ok().json(MissionHistoryResponse {
        entries,
        total_xp_from_missions: total_xp,
        next_cursor,
    }))
}

use actix_web::web as aw;

pub fn configure(cfg: &mut aw::ServiceConfig) {
    cfg.route(
        "/users/{user_id}/missions/history",
        aw::get().to(get_mission_history),
    );
}
