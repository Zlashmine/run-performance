use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

use super::models::MonthlyMission;

pub async fn get_missions_for_month(
    pool: &PgPool,
    user_id: Uuid,
    month_start: NaiveDate,
) -> Result<Vec<MonthlyMission>, AppError> {
    let rows = sqlx::query_as::<_, MonthlyMission>(
        r#"
        SELECT id, user_id, month_start, mission_type, title, description,
               target_value, current_value, xp_reward, completed_at,
               rerolled, is_boss, boss_reroll_count, created_at, updated_at
        FROM monthly_missions
        WHERE user_id = $1 AND month_start = $2
        ORDER BY is_boss ASC, created_at ASC
        "#,
    )
    .bind(user_id)
    .bind(month_start)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("get_missions_for_month error: {e}");
        AppError::Internal
    })?;

    Ok(rows)
}

pub async fn insert_missions(pool: &PgPool, missions: &[MonthlyMission]) -> Result<(), AppError> {
    for m in missions {
        sqlx::query(
            r#"
            INSERT INTO monthly_missions
                (id, user_id, month_start, mission_type, title, description,
                 target_value, current_value, xp_reward, completed_at, rerolled, is_boss,
                 boss_reroll_count, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            ON CONFLICT (user_id, month_start, mission_type) DO NOTHING
            "#,
        )
        .bind(m.id)
        .bind(m.user_id)
        .bind(m.month_start)
        .bind(&m.mission_type)
        .bind(&m.title)
        .bind(&m.description)
        .bind(m.target_value)
        .bind(m.current_value)
        .bind(m.xp_reward)
        .bind(m.completed_at)
        .bind(m.rerolled)
        .bind(m.is_boss)
        .bind(m.boss_reroll_count)
        .bind(m.created_at)
        .bind(m.updated_at)
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("insert_monthly_missions error: {e}");
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
        UPDATE monthly_missions
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
        tracing::error!("update_monthly_mission_progress error: {e}");
        AppError::Internal
    })?;

    Ok(())
}

pub async fn mark_mission_rerolled(pool: &PgPool, mission_id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE monthly_missions SET rerolled = true, updated_at = NOW() WHERE id = $1",
    )
    .bind(mission_id)
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("mark_monthly_mission_rerolled error: {e}");
        AppError::Internal
    })?;

    Ok(())
}

pub async fn get_mission_by_id(
    pool: &PgPool,
    mission_id: Uuid,
) -> Result<Option<MonthlyMission>, AppError> {
    let row = sqlx::query_as::<_, MonthlyMission>(
        r#"
        SELECT id, user_id, month_start, mission_type, title, description,
               target_value, current_value, xp_reward, completed_at,
               rerolled, is_boss, boss_reroll_count, created_at, updated_at
        FROM monthly_missions WHERE id = $1
        "#,
    )
    .bind(mission_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("get_monthly_mission_by_id error: {e}");
        AppError::Internal
    })?;

    Ok(row)
}

pub async fn delete_mission(pool: &PgPool, mission_id: Uuid) -> Result<(), AppError> {
    sqlx::query("DELETE FROM monthly_missions WHERE id = $1")
        .bind(mission_id)
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("delete_monthly_mission error: {e}");
            AppError::Internal
        })?;

    Ok(())
}
