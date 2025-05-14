use std::num::ParseFloatError;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

use chrono::NaiveDateTime;
use gpx::Waypoint;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Activity {
    pub id: Uuid,
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
    pub track_points: Option<Vec<TrackPoint>>,
}

#[derive(Deserialize, ToSchema)]
pub struct NewActivity {
    pub name: String,
    pub time: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema, sqlx:: Decode)]
pub struct TrackPoint {
    pub id: Option<Uuid>,
    pub activity_id: Uuid,
    pub latitude: String,
    pub longitude: String,
    pub elevation: f32,
    pub time: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ActivityWithTrackPoint {
    pub id: Uuid,
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

    pub trackpoint_id: Option<Uuid>,
    pub activity_id: Uuid,
    pub latitude: String,
    pub longitude: String,
    pub elevation: f32,
    pub time: String,
}

impl Activity {
    pub fn from_csv_row(row: &str) -> Result<Self, String> {
        let parts: Vec<&str> = row.split(',').collect();
        if parts.len() != 14 {
            return Err(format!("Expected 14 columns, got {}", parts.len()));
        }

        Ok(Activity {
            id: Uuid::parse_str(parts[0]).map_err(|e| e.to_string())?,
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
            track_points: None,
        })
    }
}

impl TrackPoint {
    pub async fn from_gpx_file(
        file_name: &str,
        activity_id: &Uuid,
    ) -> Result<Vec<TrackPoint>, String> {
        let file = get_cleaned_file(file_name).await?;

        match gpx::read(file) {
            Err(e) => Err(format!("Error reading GPX file: {}", e)),
            Ok(gpx) => {
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
        }
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

async fn get_cleaned_file(file_name: &str) -> Result<std::io::Cursor<String>, String> {
    let file = File::open(file_name).await.map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let mut collected_lines = Vec::new();
    let mut index = 0;

    while let Some(line) = lines.next_line().await.map_err(|e| e.to_string())? {
        if index != 10 {
            collected_lines.push(line);
        }
        index += 1;
    }

    Ok(std::io::Cursor::new(collected_lines.join("\n")))
}
