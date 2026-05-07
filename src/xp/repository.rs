use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

use super::models::{level_from_xp, AwardXpInput, UserXp, XpEvent};

pub async fn get_or_create_user_xp(db: &PgPool, user_id: Uuid) -> Result<UserXp, AppError> {
    let existing = sqlx::query_as::<_, UserXp>("SELECT * FROM user_xp WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(db)
        .await
        .map_err(AppError::from)?;

    if let Some(row) = existing {
        return Ok(row);
    }

    sqlx::query_as::<_, UserXp>(
        "INSERT INTO user_xp (user_id) VALUES ($1)
         ON CONFLICT (user_id) DO UPDATE SET updated_at = NOW()
         RETURNING *",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .map_err(AppError::from)
}

pub async fn award_xp(db: &PgPool, input: AwardXpInput) -> Result<XpEvent, AppError> {
    let mut tx = db.begin().await.map_err(AppError::from)?;

    let event = sqlx::query_as::<_, XpEvent>(
        "INSERT INTO xp_events (user_id, source_type, source_id, xp_amount, description)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING *",
    )
    .bind(input.user_id)
    .bind(&input.source_type)
    .bind(input.source_id)
    .bind(input.xp_amount)
    .bind(&input.description)
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::from)?;

    // Recompute level from new total
    let new_total: i64 = sqlx::query_scalar(
        "SELECT xp_total + $1 FROM user_xp WHERE user_id = $2",
    )
    .bind(input.xp_amount as i64)
    .bind(input.user_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::from)?
    .unwrap_or(input.xp_amount as i64);

    let (new_level, _) = level_from_xp(new_total);

    sqlx::query(
        "UPDATE user_xp
         SET xp_total = xp_total + $1,
             level = $2,
             last_awarded_at = NOW(),
             updated_at = NOW()
         WHERE user_id = $3",
    )
    .bind(input.xp_amount as i64)
    .bind(new_level)
    .bind(input.user_id)
    .execute(&mut *tx)
    .await
    .map_err(AppError::from)?;

    tx.commit().await.map_err(AppError::from)?;
    Ok(event)
}

pub async fn get_recent_events(
    db: &PgPool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<XpEvent>, AppError> {
    sqlx::query_as::<_, XpEvent>(
        "SELECT * FROM xp_events WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}

/// Seed retroactive XP for a user based on all their past activities.
/// Returns the total XP awarded.
pub async fn seed_retroactive_xp(db: &PgPool, user_id: Uuid) -> Result<i64, AppError> {
    let total_distance_km: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(distance::double precision), 0) FROM activities WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .map_err(AppError::from)?;

    let distance_km = total_distance_km; // distance column is already in km
    let xp = (distance_km * 10.0).round() as i64;

    // Insert a single retroactive event
    sqlx::query(
        "INSERT INTO xp_events (user_id, source_type, source_id, xp_amount, description)
         VALUES ($1, 'activity', NULL, $2, 'Retroactive XP for all past runs')",
    )
    .bind(user_id)
    .bind(xp as i32)
    .execute(db)
    .await
    .map_err(AppError::from)?;

    let (new_level, _) = level_from_xp(xp);

    sqlx::query(
        "UPDATE user_xp
         SET xp_total = $1,
             level = $2,
             initialized = true,
             last_awarded_at = NOW(),
             updated_at = NOW()
         WHERE user_id = $3",
    )
    .bind(xp)
    .bind(new_level)
    .bind(user_id)
    .execute(db)
    .await
    .map_err(AppError::from)?;

    Ok(xp)
}
