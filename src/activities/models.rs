use std::collections::HashMap;
use std::num::ParseFloatError;

use chrono::NaiveDateTime;
use gpx::Waypoint;
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

#[derive(Debug, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct TrackPointsResponse {
    pub track_points: Vec<TrackPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Activity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub date: chrono::NaiveDateTime,
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

#[derive(Deserialize, ToSchema, Clone, Serialize)]
pub struct NewActivity {
    pub name: String,
    pub time: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema, sqlx:: Decode)]
pub struct TrackPoint {
    pub id: Option<Uuid>,
    pub activity_id: Uuid,
    pub latitude: String,
    pub longitude: String,
    pub elevation: f32,
    pub time: String,
}

impl Activity {
    pub fn from_csv_row(row: &str, user_id: Uuid) -> Result<Self, String> {
        let parts: Vec<&str> = row.split(',').collect();

        if parts.len() != 14 {
            return Err(format!("Expected 14 columns, got {}", parts.len()));
        }

        Ok(Activity {
            id: Uuid::parse_str(parts[0]).map_err(|e| e.to_string())?,
            user_id,
            date: NaiveDateTime::parse_from_str(parts[1], "%Y-%m-%d %H:%M:%S")
                .map_err(|e| e.to_string())?,
            activity_type: parts[2].to_string(),
            name: parts[3].to_string(),
            distance: parts[4]
                .parse()
                .map_err(|e: ParseFloatError| e.to_string())?,
            duration: parts[5].to_string(),
            average_pace: parts[6]
                .replace(":", ".")
                .parse()
                .map_err(|e: ParseFloatError| e.to_string())?,
            average_speed: parts[7]
                .parse()
                .map_err(|e: ParseFloatError| e.to_string())?,
            calories: parts[8]
                .parse()
                .map_err(|e: ParseFloatError| e.to_string())?,
            climb: parts[9]
                .parse()
                .map_err(|e: ParseFloatError| e.to_string())?,
            gps_file: parts[13].to_string(),
        })
    }
}

impl TrackPoint {
    fn clean_gpx_data(data: &[u8]) -> Result<std::io::Cursor<String>, String> {
        let content = std::str::from_utf8(data).map_err(|e| e.to_string())?;
        let lines: Vec<&str> = content.lines().collect();
        let cleaned: Vec<&str> = lines
            .iter()
            .enumerate()
            .filter_map(|(i, line)| if i != 10 { Some(*line) } else { None })
            .collect();

        Ok(std::io::Cursor::new(cleaned.join("\n")))
    }

    pub async fn from_gpx_data(data: &[u8], activity_id: &Uuid) -> Result<Vec<TrackPoint>, String> {
        let cursor = TrackPoint::clean_gpx_data(data)?;

        let gpx = gpx::read(cursor).map_err(|e| format!("Error reading GPX data: {}", e))?;
        let mut track_points = Vec::new();

        for track in gpx.tracks {
            for segment in track.segments {
                for waypoint in segment.points {
                    let track_point = TrackPoint::from_waypoint(waypoint, *activity_id)?;
                    track_points.push(track_point);
                }
            }
        }

        Ok(track_points)
    }

    fn from_waypoint(waypoint: Waypoint, activity_id: Uuid) -> Result<TrackPoint, String> {
        Ok(TrackPoint {
            id: Some(Uuid::new_v4()),
            activity_id,
            latitude: waypoint.point().y().to_string(),
            longitude: waypoint.point().x().to_string(),
            elevation: waypoint.elevation.unwrap_or(0.0) as f32,
            time: waypoint.time.unwrap().format().unwrap(),
        })
    }
}
