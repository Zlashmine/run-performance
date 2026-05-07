use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Persistent XP totals and level for a user.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct UserXp {
    pub id: Uuid,
    pub user_id: Uuid,
    pub xp_total: i64,
    pub level: i16,
    pub initialized: bool,
    pub last_awarded_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

/// A single XP earn event.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct XpEvent {
    pub id: Uuid,
    pub user_id: Uuid,
    pub source_type: String,
    pub source_id: Option<Uuid>,
    pub xp_amount: i32,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

/// Response for GET /users/{id}/xp.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserXpResponse {
    pub xp_total: i64,
    pub level: i16,
    pub level_name: String,
    pub xp_for_current_level: i64,
    pub xp_for_next_level: i64,
    pub xp_in_current_level: i64,
    pub progress_percent: f64,
    pub next_level_name: String,
    pub recent_events: Vec<XpEvent>,
}

/// Used by other services internally to grant XP.
#[derive(Debug, Clone)]
pub struct AwardXpInput {
    pub user_id: Uuid,
    pub source_type: String,
    pub source_id: Option<Uuid>,
    pub xp_amount: i32,
    pub description: String,
}

pub const LEVEL_THRESHOLDS: &[(i16, &str, i64)] = &[
    (1, "Rookie", 0),
    (2, "Jogger", 500),
    (3, "Pacer", 1_500),
    (4, "Strider", 3_000),
    (5, "Racer", 5_500),
    (6, "Speedster", 9_000),
    (7, "Marathoner", 14_000),
    (8, "Elite", 22_000),
    (9, "Champion", 35_000),
    (10, "Legend", 55_000),
];

pub fn level_from_xp(xp: i64) -> (i16, &'static str) {
    LEVEL_THRESHOLDS
        .iter()
        .rev()
        .find(|(_, _, threshold)| xp >= *threshold)
        .map(|(level, name, _)| (*level, *name))
        .unwrap_or((1, "Rookie"))
}

pub fn level_bounds(xp: i64) -> (i64, i64, &'static str) {
    let (level, _) = level_from_xp(xp);
    let current_idx = LEVEL_THRESHOLDS
        .iter()
        .position(|(l, _, _)| *l == level)
        .unwrap_or(0);
    let xp_for_current = LEVEL_THRESHOLDS[current_idx].2;
    let (xp_for_next, next_name) = if current_idx + 1 < LEVEL_THRESHOLDS.len() {
        (
            LEVEL_THRESHOLDS[current_idx + 1].2,
            LEVEL_THRESHOLDS[current_idx + 1].1,
        )
    } else {
        // Already at max level — no next level
        (xp_for_current, "Legend")
    };
    (xp_for_current, xp_for_next, next_name)
}
