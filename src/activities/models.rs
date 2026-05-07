use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::aggregate::models::{ActivitiesAggregation, AggregationDTO};

#[derive(Debug, ToSchema, Deserialize)]
pub struct UploadForm {
    #[schema(format = "binary")]
    #[allow(dead_code)]
    pub files: Vec<String>, // Represented as `format: binary` in OpenAPI
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ActivitiesResponse {
    pub activities: Vec<Activity>,
    pub aggregation: Option<HashMap<String, AggregationDTO>>,
    pub time_aggregations: Option<HashMap<String, HashMap<String, ActivitiesAggregation>>>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ActivityDetailResponse {
    pub activity: Activity,
    pub track_points: Vec<TrackPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Activity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub date: NaiveDateTime,
    pub name: String,
    pub activity_type: String,
    pub distance: f32,
    pub duration: String,
    pub average_pace: f32,
    pub average_speed: f32,
    pub calories: f32,
    pub climb: f32,
    pub gps_file: String,
    /// Data source: `"runkeeper"` or `"strava"`.
    #[serde(default = "default_source")]
    pub source: String,
    /// Source-specific stable ID for deduplication (None for legacy Runkeeper rows).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
}

fn default_source() -> String {
    "runkeeper".to_string()
}

/// A GPS track point.
///
/// `latitude` and `longitude` are stored as DOUBLE PRECISION in the DB
/// (migration 20250522000001).  `time` is TIMESTAMPTZ.
/// `speed` is DOUBLE PRECISION (m/s), nullable — populated on new uploads only.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct TrackPoint {
    pub id: Option<Uuid>,
    pub activity_id: Uuid,
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: f32,
    pub time: DateTime<Utc>,
    pub speed: Option<f64>,
}

/// A single point in the geographic heatmap grid.
///
/// Coordinates are rounded to 4 decimal places (~11 m grid cells).
#[derive(Debug, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct HeatmapPoint {
    pub lat: f64,
    pub lon: f64,
    pub weight: i64,
}

/// Optional query parameters for the heatmap endpoint.
#[derive(Debug, Deserialize, ToSchema)]
pub struct HeatmapQuery {
    pub activity_type: Option<String>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
}

/// Response returned after a successful upload.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UploadResponse {
    /// Number of GPX files processed.
    pub processed: u32,
    /// Total XP earned from this upload batch.
    pub xp_earned: i64,
    /// Set to the new level name if the user levelled up during this upload.
    pub new_level: Option<String>,
    /// Achievements newly unlocked during this upload batch.
    pub newly_unlocked_achievements: Vec<crate::achievements::models::UnlockedAchievementSummary>,
    /// Personal records set or improved during this upload batch.
    pub new_prs: Vec<crate::personal_records::models::PrCategorySummary>,
    /// Weekly missions completed during this upload batch.
    pub completed_missions: Vec<crate::weekly_missions::models::CompletedMissionSummary>,
}
