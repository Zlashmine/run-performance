use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Distance categories with optional range bounds (in metres).
/// `None` bounds for `longest_run` mean "any distance" — winner is the longest.
pub const CATEGORIES: &[(&str, Option<f64>, Option<f64>)] = &[
    ("5k",            Some(4_750.0), Some(5_250.0)),
    ("10k",           Some(9_500.0), Some(10_500.0)),
    ("half_marathon", Some(20_600.0), Some(21_600.0)),
    ("marathon",      Some(41_200.0), Some(43_200.0)),
    ("longest_run",   None, None),
];

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct PersonalRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub category: String,
    pub activity_id: Option<Uuid>,
    pub distance_m: f64,
    pub duration_seconds: i64,
    pub pace_seconds_per_km: f64,
    pub achieved_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// One category row in the full PR response.  All metric fields are `Option`
/// because the user may not yet have a PR for that category.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PersonalRecordSummary {
    pub category: String,
    pub category_display: String,
    pub distance_m: Option<f64>,
    pub duration_seconds: Option<i64>,
    pub pace_seconds_per_km: Option<f64>,
    pub achieved_at: Option<DateTime<Utc>>,
    pub activity_id: Option<Uuid>,
}

/// Response returned from `GET /users/{id}/personal_records`.
#[derive(Debug, Serialize, ToSchema)]
pub struct PersonalRecordsResponse {
    pub records: Vec<PersonalRecordSummary>,
}

/// Compact summary included in the upload response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PrCategorySummary {
    pub category: String,
    pub category_display: String,
    pub pace_seconds_per_km: f64,
    /// true = first time recording a PR in this category.
    pub is_first_pr: bool,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

pub fn category_display(slug: &str) -> &'static str {
    match slug {
        "5k"            => "5K",
        "10k"           => "10K",
        "half_marathon" => "Half Marathon",
        "marathon"      => "Marathon",
        "longest_run"   => "Longest Run",
        _               => "Unknown",
    }
}

#[allow(dead_code)]
pub fn format_pace(pace_secs: f64) -> String {
    let mins = (pace_secs / 60.0).floor() as u32;
    let secs = (pace_secs % 60.0).round() as u32;
    format!("{}:{:02}/km", mins, secs)
}

/// Parse "H:MM:SS" or "MM:SS" duration string → total seconds.
pub fn parse_duration_to_secs(s: &str) -> i64 {
    let parts: Vec<&str> = s.split(':').collect();
    match parts.as_slice() {
        [h, m, sec] => {
            let h: i64 = h.parse().unwrap_or(0);
            let m: i64 = m.parse().unwrap_or(0);
            let sec: i64 = sec.parse().unwrap_or(0);
            h * 3600 + m * 60 + sec
        }
        [m, sec] => {
            let m: i64 = m.parse().unwrap_or(0);
            let sec: i64 = sec.parse().unwrap_or(0);
            m * 60 + sec
        }
        _ => 0,
    }
}
