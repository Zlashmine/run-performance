/// SQL layer for the goals domain.
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

use super::models::{GoalRequirement, UserGoal};

// ─── Read ─────────────────────────────────────────────────────────────────────

pub async fn find_goals_for_user(
    db: &PgPool,
    user_id: Uuid,
) -> Result<Vec<(UserGoal, Vec<GoalRequirement>)>, AppError> {
    // Load goals in one query.
    let goals = sqlx::query_as::<_, UserGoal>(
        "SELECT * FROM goals WHERE user_id = $1 ORDER BY created_at ASC",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(AppError::from)?;

    if goals.is_empty() {
        return Ok(vec![]);
    }

    // Batch-load all requirements for these goals in a single query.
    let goal_ids: Vec<Uuid> = goals.iter().map(|g| g.id).collect();
    let requirements = sqlx::query_as::<_, GoalRequirement>(
        "SELECT * FROM goal_requirements WHERE goal_id = ANY($1) ORDER BY category DESC, created_at ASC",
    )
    .bind(&goal_ids)
    .fetch_all(db)
    .await
    .map_err(AppError::from)?;

    // Group requirements by goal_id.
    let mut req_map: std::collections::HashMap<Uuid, Vec<GoalRequirement>> =
        std::collections::HashMap::new();
    for req in requirements {
        req_map.entry(req.goal_id).or_default().push(req);
    }

    Ok(goals
        .into_iter()
        .map(|g| {
            let reqs = req_map.remove(&g.id).unwrap_or_default();
            (g, reqs)
        })
        .collect())
}

/// Count "active" goal slots for a user.
/// Forever goals that are permanently completed do NOT count toward the limit.
pub async fn count_active_slots(db: &PgPool, user_id: Uuid) -> Result<i64, AppError> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM goals
         WHERE user_id = $1
           AND NOT (timeframe = 'forever' AND completed_at IS NOT NULL)",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .map_err(AppError::from)?;
    Ok(row.0)
}

pub async fn find_goal_by_id(
    db: &PgPool,
    goal_id: Uuid,
) -> Result<Option<UserGoal>, AppError> {
    sqlx::query_as::<_, UserGoal>("SELECT * FROM goals WHERE id = $1")
        .bind(goal_id)
        .fetch_optional(db)
        .await
        .map_err(AppError::from)
}

// ─── Write ────────────────────────────────────────────────────────────────────

pub async fn insert_goal(
    db: &PgPool,
    user_id: Uuid,
    name: &str,
    description: Option<&str>,
    timeframe: &str,
    period_key: &str,
    target_value: f64,
    xp_reward: i32,
) -> Result<UserGoal, AppError> {
    sqlx::query_as::<_, UserGoal>(
        r#"
        INSERT INTO goals
            (user_id, name, description, timeframe, period_key, target_value, xp_reward)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(name)
    .bind(description)
    .bind(timeframe)
    .bind(period_key)
    .bind(target_value)
    .bind(xp_reward)
    .fetch_one(db)
    .await
    .map_err(AppError::from)
}

pub async fn insert_goal_requirements(
    db: &PgPool,
    goal_id: Uuid,
    requirements: &[(String, String, Option<f64>, serde_json::Value)],
) -> Result<(), AppError> {
    for (category, req_type, value, params) in requirements {
        sqlx::query(
            r#"
            INSERT INTO goal_requirements (goal_id, category, requirement_type, value, params)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(goal_id)
        .bind(category)
        .bind(req_type)
        .bind(value)
        .bind(params)
        .execute(db)
        .await
        .map_err(AppError::from)?;
    }
    Ok(())
}

pub async fn delete_goal(db: &PgPool, goal_id: Uuid) -> Result<(), AppError> {
    // ON DELETE CASCADE handles goal_requirements automatically.
    sqlx::query("DELETE FROM goals WHERE id = $1")
        .bind(goal_id)
        .execute(db)
        .await
        .map_err(AppError::from)?;
    Ok(())
}

pub async fn update_goal_progress(
    db: &PgPool,
    goal_id: Uuid,
    current_value: f64,
    period_key: &str,
    completed_at: Option<DateTime<Utc>>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE goals
           SET current_value = $2,
               period_key    = $3,
               completed_at  = $4,
               updated_at    = NOW()
         WHERE id = $1
        "#,
    )
    .bind(goal_id)
    .bind(current_value)
    .bind(period_key)
    .bind(completed_at)
    .execute(db)
    .await
    .map_err(AppError::from)?;
    Ok(())
}
