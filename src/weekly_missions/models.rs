use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// Re-export the shared summary type so external code can import from either location.
pub use crate::missions::common::CompletedMissionSummary;

/// A single weekly mission for a user.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyMission {
    pub id: Uuid,
    pub user_id: Uuid,
    pub week_start: NaiveDate,
    pub mission_type: String,
    pub title: String,
    pub description: String,
    pub target_value: f64,
    pub current_value: f64,
    pub xp_reward: i32,
    pub completed_at: Option<DateTime<Utc>>,
    pub rerolled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Response for GET /users/{user_id}/weekly_missions.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyMissionsResponse {
    pub week_start: NaiveDate,
    pub missions: Vec<WeeklyMission>,
    /// True if the user has not yet used their one free reroll this week.
    pub can_reroll: bool,
}
