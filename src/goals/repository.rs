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

pub async fn find_goal_by_id(db: &PgPool, goal_id: Uuid) -> Result<Option<UserGoal>, AppError> {
    sqlx::query_as::<_, UserGoal>("SELECT * FROM goals WHERE id = $1")
        .bind(goal_id)
        .fetch_optional(db)
        .await
        .map_err(AppError::from)
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
