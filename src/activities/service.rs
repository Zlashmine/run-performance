/// Business-logic layer for the activities domain.
///
/// Orchestrates between repository (SQL) and parser (file parsing).
/// No SQL and no HTTP here.
use std::collections::HashMap;

use sqlx::PgPool;
use uuid::Uuid;

use crate::{aggregate::aggregate_activities, error::AppError};

use super::{
    models::{ActivitiesResponse, Activity, ActivityDetailResponse, TrackPoint},
    parser, repository,
};

pub async fn get_activities(db: &PgPool, user_id: Uuid) -> Result<ActivitiesResponse, AppError> {
    let activities = repository::find_all_by_user(db, user_id).await?;
    let (aggregation, time_aggregations) = aggregate_activities(&activities);

    Ok(ActivitiesResponse {
        activities,
        aggregation: Some(aggregation),
        time_aggregations: Some(time_aggregations),
    })
}

pub async fn get_activity_detail(
    db: &PgPool,
    activity_id: Uuid,
) -> Result<ActivityDetailResponse, AppError> {
    let activity = repository::find_by_id(db, activity_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let track_points = repository::find_trackpoints(db, activity_id).await?;

    Ok(ActivityDetailResponse {
        activity,
        track_points,
    })
}

pub async fn get_trackpoints(db: &PgPool, activity_id: Uuid) -> Result<Vec<TrackPoint>, AppError> {
    repository::find_trackpoints(db, activity_id).await
}

/// Process an upload: parse CSV rows and GPX files, then persist.
pub async fn upload(
    db: &PgPool,
    user_id: Uuid,
    csv_lines: Vec<String>,
    gpx_files: HashMap<String, Vec<u8>>,
) -> usize {
    // Parse activities from CSV rows (synchronous — CPU only, no I/O).
    let activities: Vec<Activity> = csv_lines
        .iter()
        .filter_map(|row| match parser::parse_csv_row(row, user_id) {
            Ok(a) => Some(a),
            Err(e) => {
                tracing::warn!("Skipping CSV row: {}", e);
                None
            }
        })
        .collect();

    // Persist activities (ON CONFLICT DO NOTHING for duplicates).
    repository::insert_activities(db, &activities).await;

    // Parse GPX files for each activity (synchronous — CPU only, no I/O).
    let mut trackpoints_map: HashMap<Uuid, Vec<TrackPoint>> = HashMap::new();
    for activity in &activities {
        if let Some(gpx_data) = gpx_files.get(&activity.gps_file) {
            match parser::parse_gpx(gpx_data, activity.id) {
                Ok(tps) => {
                    trackpoints_map.insert(activity.id, tps);
                }
                Err(e) => {
                    tracing::warn!("Skipping GPX for {}: {}", activity.gps_file, e);
                }
            }
        }
    }

    let gpx_count = trackpoints_map.len();
    repository::insert_trackpoints(db, &trackpoints_map).await;
    gpx_count
}
