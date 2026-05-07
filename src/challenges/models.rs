use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::challenges::{ChallengeStatus, RequirementType};

// ─── Database row types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Challenge {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_recurring: bool,
    pub recurrence_period: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub status: ChallengeStatus,           // lifecycle state
    pub is_public: bool,                   // discoverable by other users
    pub parent_challenge_id: Option<Uuid>, // set when cloned via opt-in
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ChallengeWorkout {
    pub id: Uuid,
    pub challenge_id: Uuid,
    pub position: i32,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct WorkoutRequirement {
    pub id: Uuid,
    pub challenge_workout_id: Uuid,
    pub requirement_type: RequirementType,
    pub value: Option<f64>,
    #[schema(value_type = Object)]
    pub params: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct WorkoutLink {
    pub id: Uuid,
    pub challenge_workout_id: Uuid,
    pub activity_id: Option<Uuid>,
    pub state: String,
    pub linked_at: DateTime<Utc>,
}

// ─── Request DTOs ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateChallengeRequest {
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_recurring: Option<bool>,
    pub recurrence_period: Option<String>,
    /// Challenge start date (required to activate auto-progression).
    pub started_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateChallengeRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_recurring: Option<bool>,
    pub recurrence_period: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWorkoutRequest {
    pub name: String,
    pub description: Option<String>,
    /// 1-based position; if omitted, appended at the end.
    pub position: Option<i32>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateWorkoutRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ReorderWorkoutRequest {
    pub new_position: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddRequirementRequest {
    pub requirement_type: RequirementType,
    pub value: Option<f64>,
    #[schema(value_type = Object)]
    pub params: Option<serde_json::Value>,
}

/// Body for POST /challenges/:id/activate.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ActivateChallengeRequest {
    /// If true, the challenge becomes publicly discoverable once active.
    pub is_public: Option<bool>,
}

/// Body for POST /challenges/:id/opt-in.
#[derive(Debug, Deserialize, ToSchema)]
pub struct OptInRequest {
    /// ID of the user opting in (follows existing pattern — no JWT middleware).
    pub user_id: Uuid,
}

// ─── Query params ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct ListChallengesParams {
    pub user_id: Uuid,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ListPublicChallengesParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ─── Response / aggregated types ─────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct ParticipantsResponse {
    pub count: i64,
    pub participants: Vec<Challenge>,
}

// ─── Response / enriched types ────────────────────────────────────────────────

/// Lightweight summary returned by the list endpoints.
/// Contains workout progress counts without requiring N+1 detail fetches.
#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct ChallengeSummary {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub challenge: Challenge,
    /// Total number of workout slots in this challenge.
    pub workout_count: i64,
    /// Number of workouts with a completed link row (all link rows are 'completed' in DB).
    pub completed_count: i64,
    /// Dominant activity type derived from requirements (e.g. "running"). None if mixed or no requirements.
    pub primary_activity_type: Option<String>,
    /// Participant count — Some for public/template challenges, None for private.
    pub participant_count: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WorkoutWithDetails {
    #[serde(flatten)]
    pub workout: ChallengeWorkout,
    pub requirements: Vec<WorkoutRequirement>,
    pub link: Option<WorkoutLink>,
    /// Computed from the linked activity and requirements.
    pub state: WorkoutState,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChallengeDetail {
    #[serde(flatten)]
    pub challenge: Challenge,
    pub workouts: Vec<WorkoutWithDetails>,
    /// Some(n) for public challenges; None for private.
    pub participant_count: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkoutState {
    NotStarted,
    Completed,
    Failed,
}

/// A single row in the challenge leaderboard.
#[derive(Debug, Serialize, ToSchema)]
pub struct LeaderboardEntry {
    pub rank: i64,
    pub user_id: Uuid,
    pub display_name: String,
    pub completed_workouts: i64,
    pub total_workouts: i64,
    pub completion_percent: f64,
}

/// Response for GET /challenges/{id}/leaderboard.
#[derive(Debug, Serialize, ToSchema)]
pub struct LeaderboardResponse {
    pub challenge_id: Uuid,
    pub challenge_name: String,
    pub total_participants: i64,
    pub entries: Vec<LeaderboardEntry>,
}

// ─── Goal Wizard ──────────────────────────────────────────────────────────────

/// Goal types supported by the plan generator.
/// - `sub2_half_marathon`: 12-week plan targeting a sub-2:00 21.1 km race
/// - `5k_improvement`:    6-week plan targeting a personal-best 5 km race
#[derive(Debug, Deserialize, ToSchema)]
pub struct GenerateChallengeRequest {
    pub user_id: Uuid,
    /// "sub2_half_marathon" | "5k_improvement"
    pub goal_type: String,
    /// Target pace in M.SS format (e.g. 5.41 = 5:41/km).
    /// Defaults to 5.41 for half marathon and 5.00 for 5 km.
    pub target_pace_mss: Option<f64>,
    /// Override the plan length in weeks.
    pub weeks: Option<u32>,
    /// Optional name for the generated challenge.
    pub name: Option<String>,
}

// ─── Validation helpers ───────────────────────────────────────────────────────

const VALID_RECURRENCE_PERIODS: &[&str] = &["daily", "weekly", "monthly"];

impl CreateChallengeRequest {
    pub fn validate(&self) -> Result<(), String> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err("Challenge name must not be empty".into());
        }
        if name.len() > 200 {
            return Err("Challenge name must be 200 characters or fewer".into());
        }
        if let Some(ref period) = self.recurrence_period {
            if !VALID_RECURRENCE_PERIODS.contains(&period.as_str()) {
                return Err(format!(
                    "Invalid recurrence_period '{}'. Must be one of: {}",
                    period,
                    VALID_RECURRENCE_PERIODS.join(", ")
                ));
            }
        }
        if let (Some(started), Some(ends)) = (self.started_at, self.ends_at) {
            if ends <= started {
                return Err("ends_at must be after started_at".into());
            }
        }
        Ok(())
    }
}

impl CreateWorkoutRequest {
    pub fn validate(&self) -> Result<(), String> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err("Workout name must not be empty".into());
        }
        if name.len() > 200 {
            return Err("Workout name must be 200 characters or fewer".into());
        }
        if let Some(pos) = self.position {
            if pos < 1 {
                return Err("Position must be >= 1".into());
            }
        }
        Ok(())
    }
}
