use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

use super::models::WeeklyMission;

pub async fn get_missions_for_week(
    pool: &PgPool,
    user_id: Uuid,
    week_start: NaiveDate,
) -> Result<Vec<WeeklyMission>, AppError> {
    let rows = sqlx::query_as::<_, WeeklyMission>(
        r#"
        SELECT id, user_id, week_start, mission_type, title, description,
               target_value, current_value, xp_reward, completed_at,
               rerolled, created_at, updated_at
        FROM weekly_missions
        WHERE user_id = $1 AND week_start = $2
        ORDER BY created_at ASC
        "#,
    )
    .bind(user_id)
    .bind(week_start)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("get_missions_for_week error: {e}");
        AppError::Internal
    })?;

    Ok(rows)
}

pub async fn insert_missions(pool: &PgPool, missions: &[WeeklyMission]) -> Result<(), AppError> {
    for m in missions {
        sqlx::query(
            r#"
            INSERT INTO weekly_missions
                (id, user_id, week_start, mission_type, title, description,
                 target_value, current_value, xp_reward, completed_at, rerolled, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (user_id, week_start, mission_type) DO NOTHING
            "#,
        )
        .bind(m.id)
        .bind(m.user_id)
        .bind(m.week_start)
        .bind(&m.mission_type)
        .bind(&m.title)
        .bind(&m.description)
        .bind(m.target_value)
        .bind(m.current_value)
        .bind(m.xp_reward)
        .bind(m.completed_at)
        .bind(m.rerolled)
        .bind(m.created_at)
        .bind(m.updated_at)
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("insert_missions error: {e}");
            AppError::Internal
        })?;
    }
    Ok(())
}

pub async fn update_mission_progress(
    pool: &PgPool,
    mission_id: Uuid,
    current_value: f64,
    completed_at: Option<DateTime<Utc>>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE weekly_missions
        SET current_value = $2, completed_at = $3, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(mission_id)
    .bind(current_value)
    .bind(completed_at)
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("update_mission_progress error: {e}");
        AppError::Internal
    })?;

    Ok(())
}

pub async fn mark_mission_rerolled(pool: &PgPool, mission_id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE weekly_missions SET rerolled = true, updated_at = NOW() WHERE id = $1",
    )
    .bind(mission_id)
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("mark_mission_rerolled error: {e}");
        AppError::Internal
    })?;

    Ok(())
}

pub async fn get_mission_by_id(pool: &PgPool, mission_id: Uuid) -> Result<Option<WeeklyMission>, AppError> {
    let row = sqlx::query_as::<_, WeeklyMission>(
        r#"
        SELECT id, user_id, week_start, mission_type, title, description,
               target_value, current_value, xp_reward, completed_at,
               rerolled, created_at, updated_at
        FROM weekly_missions WHERE id = $1
        "#,
    )
    .bind(mission_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("get_mission_by_id error: {e}");
        AppError::Internal
    })?;

    Ok(row)
}

pub async fn delete_mission(pool: &PgPool, mission_id: Uuid) -> Result<(), AppError> {
    sqlx::query("DELETE FROM weekly_missions WHERE id = $1")
        .bind(mission_id)
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("delete_mission error: {e}");
            AppError::Internal
        })?;

    Ok(())
}
