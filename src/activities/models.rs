use std::collections::HashMap;

use chrono::{DateTime, NaiveDateTime, Utc};
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
}

/// A GPS track point.
///
/// `latitude` and `longitude` are stored as DOUBLE PRECISION in the DB
/// (migration 20250522000001).  `time` is TIMESTAMPTZ.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct TrackPoint {
    pub id: Option<Uuid>,
    pub activity_id: Uuid,
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: f32,
    pub time: DateTime<Utc>,
}
