use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use super::requirement_type::{GoalFilterType, GoalMetricType};

// ─── DB row types ─────────────────────────────────────────────────────────────

/// Raw row from the `goals` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct UserGoal {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub timeframe: String,
    pub period_key: String,
    pub current_value: f64,
    pub target_value: f64,
    pub completed_at: Option<DateTime<Utc>>,
    pub xp_reward: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Raw row from the `goal_requirements` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct GoalRequirement {
    pub id: Uuid,
    pub goal_id: Uuid,
    pub category: String,
    pub requirement_type: String,
    pub value: Option<f64>,
    #[schema(value_type = Object)]
    pub params: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

// ─── Response / view types ────────────────────────────────────────────────────

/// A goal with its requirements, returned by GET /users/{user_id}/goals.
///
/// If the goal's period has rolled over since the last upload the
/// `current_value` and `completed_at` fields are returned as 0 / None
/// so the UI shows a fresh slate without requiring a DB write.
#[derive(Debug, Serialize, ToSchema)]
pub struct UserGoalResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub timeframe: String,
    pub period_key: String,
    pub current_value: f64,
    pub target_value: f64,
    pub completed_at: Option<DateTime<Utc>>,
    pub xp_reward: i32,
    pub requirements: Vec<GoalRequirement>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Minimal summary included in the UploadResponse when a goal is completed.
#[derive(Debug, Serialize, ToSchema)]
pub struct CompletedGoalSummary {
    pub goal_id: Uuid,
    pub name: String,
    pub xp_earned: i32,
}

// ─── Request DTOs ─────────────────────────────────────────────────────────────

/// One requirement inside a CreateGoalRequest.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateGoalRequirementRequest {
    /// "metric" or "filter"
    pub category: String,
    pub requirement_type: String,
    pub value: Option<f64>,
    #[serde(default = "serde_json::Value::default")]
    pub params: serde_json::Value,
}

/// Payload for POST /users/{user_id}/goals.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateGoalRequest {
    pub name: String,
    pub description: Option<String>,
    /// "monthly" | "yearly" | "forever"
    pub timeframe: String,
    pub target_value: f64,
    pub xp_reward: Option<i32>,
    pub requirements: Vec<CreateGoalRequirementRequest>,
}

/// Parsed and validated metric + filter types extracted from a CreateGoalRequest.
/// Used internally by the service layer.
pub struct ParsedRequirements {
    pub metric: GoalMetricType,
    pub filters: Vec<(GoalFilterType, Option<f64>, serde_json::Value)>,
}
