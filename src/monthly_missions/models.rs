use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// A single monthly mission (regular or boss) for a user.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct MonthlyMission {
    pub id: Uuid,
    pub user_id: Uuid,
    pub month_start: NaiveDate,
    pub mission_type: String,
    pub title: String,
    pub description: String,
    pub target_value: f64,
    pub current_value: f64,
    pub xp_reward: i32,
    pub completed_at: Option<DateTime<Utc>>,
    pub rerolled: bool,
    pub is_boss: bool,
    pub boss_reroll_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Response for GET /users/{user_id}/monthly_missions.
#[derive(Debug, Serialize, ToSchema)]
pub struct MonthlyMissionsResponse {
    pub month_start: NaiveDate,
    /// The 3 regular (non-boss) missions.
    pub missions: Vec<MonthlyMission>,
    /// The boss battle mission for the month. `None` only during a transient generation failure.
    pub boss: Option<MonthlyMission>,
    /// True if the user has not yet used their one free reroll this month (regular missions only).
    pub can_reroll: bool,
    /// True if the boss mission can still be rerolled (up to 2 times per month).
    pub can_reroll_boss: bool,
}
