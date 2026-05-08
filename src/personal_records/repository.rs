use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

use super::models::PersonalRecord;

pub async fn get_all_prs(db: &PgPool, user_id: Uuid) -> Result<Vec<PersonalRecord>, AppError> {
    sqlx::query_as::<_, PersonalRecord>(
        "SELECT * FROM personal_records WHERE user_id = $1 ORDER BY category",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}

pub async fn get_pr(
    db: &PgPool,
    user_id: Uuid,
    category: &str,
) -> Result<Option<PersonalRecord>, AppError> {
    sqlx::query_as::<_, PersonalRecord>(
        "SELECT * FROM personal_records WHERE user_id = $1 AND category = $2",
    )
    .bind(user_id)
    .bind(category)
    .fetch_optional(db)
    .await
    .map_err(AppError::from)
}

/// Upsert a PR.
///
/// For categories other than `longest_run`: only updates if the new pace is
/// strictly faster (lower seconds/km) than the stored record.
///
/// For `longest_run`: only updates if the new distance is strictly greater.
///
/// Returns `Some(record)` if the record was inserted or updated, `None` if the
/// existing record was better.
#[allow(clippy::too_many_arguments)]
pub async fn upsert_pr(
    db: &PgPool,
    user_id: Uuid,
    category: &str,
    activity_id: Uuid,
    distance_m: f64,
    duration_seconds: i64,
    pace_seconds_per_km: f64,
    achieved_at: DateTime<Utc>,
) -> Result<Option<PersonalRecord>, AppError> {
    let condition_column = if category == "longest_run" {
        // For longest_run, update when the new distance is greater.
        "distance_m"
    } else {
        "pace_seconds_per_km"
    };

    // Two separate queries because the WHERE clause in the DO UPDATE differs.
    let row = if category == "longest_run" {
        sqlx::query_as::<_, PersonalRecord>(
            "INSERT INTO personal_records \
                (user_id, category, activity_id, distance_m, duration_seconds, pace_seconds_per_km, achieved_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (user_id, category) DO UPDATE
                SET activity_id          = EXCLUDED.activity_id,
                    distance_m           = EXCLUDED.distance_m,
                    duration_seconds     = EXCLUDED.duration_seconds,
                    pace_seconds_per_km  = EXCLUDED.pace_seconds_per_km,
                    achieved_at          = EXCLUDED.achieved_at,
                    updated_at           = NOW()
                WHERE EXCLUDED.distance_m > personal_records.distance_m
             RETURNING *",
        )
        .bind(user_id)
        .bind(category)
        .bind(activity_id)
        .bind(distance_m)
        .bind(duration_seconds)
        .bind(pace_seconds_per_km)
        .bind(achieved_at)
        .fetch_optional(db)
        .await
        .map_err(AppError::from)?
    } else {
        sqlx::query_as::<_, PersonalRecord>(
            "INSERT INTO personal_records \
                (user_id, category, activity_id, distance_m, duration_seconds, pace_seconds_per_km, achieved_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (user_id, category) DO UPDATE
                SET activity_id          = EXCLUDED.activity_id,
                    distance_m           = EXCLUDED.distance_m,
                    duration_seconds     = EXCLUDED.duration_seconds,
                    pace_seconds_per_km  = EXCLUDED.pace_seconds_per_km,
                    achieved_at          = EXCLUDED.achieved_at,
                    updated_at           = NOW()
                WHERE EXCLUDED.pace_seconds_per_km < personal_records.pace_seconds_per_km
             RETURNING *",
        )
        .bind(user_id)
        .bind(category)
        .bind(activity_id)
        .bind(distance_m)
        .bind(duration_seconds)
        .bind(pace_seconds_per_km)
        .bind(achieved_at)
        .fetch_optional(db)
        .await
        .map_err(AppError::from)?
    };

    // Suppress unused variable warning from condition_column (used only for docs).
    let _ = condition_column;

    Ok(row)
}
