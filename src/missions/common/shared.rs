use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Compact summary of a completed mission — included in upload response.
/// Shared by both weekly and monthly mission modules.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CompletedMissionSummary {
    pub id: Uuid,
    pub title: String,
    pub xp_reward: i32,
    /// True for boss battle missions — determines dramatic toast styling on the frontend.
    pub is_boss: bool,
}

/// Returns true when a mission is complete.
/// Handles inverted (lower-is-better) logic for pace missions.
pub fn is_mission_complete(mission_type: &str, current: f64, target: f64) -> bool {
    match mission_type {
        "run_sub_pace" | "boss_speed_demon" => current <= target,
        _ => current >= target,
    }
}

/// Format a pace in seconds/km as "M:SS".
pub fn format_pace_str(secs: f64) -> String {
    let mins = (secs / 60.0).floor() as u32;
    let s = (secs % 60.0).round() as u32;
    format!("{mins}:{s:02}")
}

/// Map a PostgreSQL day-of-week integer (0=Sunday … 6=Saturday) to a name.
pub fn dow_name(dow: u32) -> &'static str {
    match dow {
        0 => "Sunday",
        1 => "Monday",
        2 => "Tuesday",
        3 => "Wednesday",
        4 => "Thursday",
        5 => "Friday",
        6 => "Saturday",
        _ => "Unknown",
    }
}
