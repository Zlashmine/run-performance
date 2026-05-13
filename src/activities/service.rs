/// Business-logic layer for the activities domain.
///
/// Orchestrates between repository (SQL) and parser (file parsing).
/// No SQL and no HTTP here.
use std::collections::HashMap;

use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    achievements,
    aggregate::aggregate_activities,
    error::AppError,
    monthly_missions,
    personal_records,
    sync::normalized::NormalizedActivity,
    weekly_missions,
    xp::{
        models::AwardXpInput,
        service as xp_service,
    },
};

use super::{
    models::{ActivitiesResponse, Activity, ActivityDetailResponse, HeatmapPoint, TrackPoint, UploadResponse},
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
    user_id: Uuid,
) -> Result<ActivityDetailResponse, AppError> {
    let activity = repository::find_by_id(db, activity_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if activity.user_id != user_id {
        return Err(AppError::NotFound);
    }

    let track_points = repository::find_trackpoints(db, activity_id).await?;

    Ok(ActivityDetailResponse {
        activity,
        track_points,
    })
}

pub async fn get_trackpoints(
    db: &PgPool,
    activity_id: Uuid,
    user_id: Uuid,
) -> Result<Vec<TrackPoint>, AppError> {
    let activity = repository::find_by_id(db, activity_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if activity.user_id != user_id {
        return Err(AppError::NotFound);
    }

    repository::find_trackpoints(db, activity_id).await
}

/// Process an upload: parse CSV rows and GPX files, then persist.
pub async fn upload(
    db: &PgPool,
    user_id: Uuid,
    csv_lines: Vec<String>,
    gpx_files: HashMap<String, Vec<u8>>,
) -> UploadResponse {
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

    let _gpx_count = trackpoints_map.len() as u32;
    repository::insert_trackpoints(db, &trackpoints_map).await;

    // Build activity IDs list for the pipeline step.
    let activity_ids: Vec<Uuid> = activities.iter().map(|a| a.id).collect();

    // Run the XP / achievement / PR / mission pipeline on the newly inserted rows.
    run_post_ingest_pipeline(db, user_id, &activity_ids, &activities).await
}

/// Core ingestion pipeline for activities arriving from any data source.
///
/// Called by `upload()` (Runkeeper) and by `strava::sync` (Strava).
/// Inserts activities that are not already present in the DB, then runs the
/// XP / achievement / PR / mission pipelines on only the *newly* inserted rows.
///
/// Returns an `UploadResponse` summarising what was earned/unlocked.
pub async fn ingest_activities(
    db: &PgPool,
    user_id: Uuid,
    activities: &[NormalizedActivity],
) -> UploadResponse {
    // Insert activities, collecting the IDs of rows that were actually new.
    let inserted_ids = repository::insert_activities_from_source(db, user_id, activities).await;

    if inserted_ids.is_empty() {
        return UploadResponse {
            processed: 0,
            xp_earned: 0,
            new_level: None,
            newly_unlocked_achievements: vec![],
            new_prs: vec![],
            completed_missions: vec![],
        };
    }

    // Insert track points for every newly inserted activity.
    let id_set: std::collections::HashSet<Uuid> = inserted_ids.iter().copied().collect();
    for activity in activities {
        if let Some(ext_id) = &activity.external_id {
            // Look up the DB-assigned UUID for this activity.
            if let Ok(Some(db_id)) = repository::find_id_by_external(db, user_id, &activity.source, ext_id).await {
                if id_set.contains(&db_id) {
                    repository::insert_normalized_trackpoints(db, db_id, &activity.track_points).await;
                }
            }
        } else {
            // For source without external_id (legacy Runkeeper), skip track points here —
            // the existing upload() path already handles them.
        }
    }

    // Re-fetch the Activity structs for the newly inserted rows so we can
    // pass typed data into the downstream pipelines.
    let db_activities = match repository::find_activities_by_ids(db, &inserted_ids).await {
        Ok(map) => {
            inserted_ids.iter().filter_map(|id| map.get(id).cloned()).collect::<Vec<_>>()
        }
        Err(e) => {
            tracing::warn!("Could not fetch inserted activities for pipeline: {e}");
            return UploadResponse {
                processed: inserted_ids.len() as u32,
                xp_earned: 0,
                new_level: None,
                newly_unlocked_achievements: vec![],
                new_prs: vec![],
                completed_missions: vec![],
            };
        }
    };

    run_post_ingest_pipeline(db, user_id, &inserted_ids, &db_activities).await
}

/// Shared XP / achievement / PR / mission pipeline.
///
/// Runs after activities have been persisted. Takes the already-fetched
/// `Activity` structs to avoid an extra DB round-trip.
async fn run_post_ingest_pipeline(
    db: &PgPool,
    user_id: Uuid,
    _activity_ids: &[Uuid],
    activities: &[Activity],
) -> UploadResponse {
    // Record XP level before awarding so we can detect level-up.
    let level_before = xp_service::get_user_xp_summary(db, user_id)
        .await
        .map(|s| s.level)
        .unwrap_or(1);

    // Award XP for each activity: 10 XP per km.
    let mut xp_earned: i64 = 0;
    for activity in activities {
        let distance_km = activity.distance as f64; // already in km
        let xp_amount = (distance_km * 10.0).round() as i32;
        if xp_amount > 0 {
            let input = AwardXpInput {
                user_id,
                source_type: "activity".to_string(),
                source_id: Some(activity.id),
                xp_amount,
                description: format!("{:.1} km run", distance_km),
            };
            if let Err(e) = xp_service::award_xp(db, input).await {
                tracing::warn!("Failed to award XP for activity {}: {e}", activity.id);
            } else {
                xp_earned += xp_amount as i64;
            }
        }
    }

    // Detect level-up
    let new_level = if xp_earned > 0 {
        xp_service::get_user_xp_summary(db, user_id)
            .await
            .ok()
            .and_then(|s| {
                if s.level > level_before {
                    Some(s.level_name)
                } else {
                    None
                }
            })
    } else {
        None
    };

    // Check & unlock achievements for each activity.
    let mut all_unlocked = Vec::new();
    for activity in activities {
        let distance_m = activity.distance as f64 * 1000.0; // km → m
        let pace = activity.average_pace as f64;
        let start = activity.date.and_utc();
        match achievements::service::check_and_unlock_achievements(
            db,
            user_id,
            activity.id,
            distance_m,
            pace,
            start,
        )
        .await
        {
            Ok(unlocked) => all_unlocked.extend(unlocked),
            Err(e) => tracing::warn!("Achievement check failed for activity {}: {e}", activity.id),
        }
    }

    // Check personal records for each activity.
    let mut all_new_prs = Vec::new();
    for activity in activities {
        let distance_m = activity.distance as f64 * 1000.0; // km → m
        let start = activity.date.and_utc();
        match personal_records::service::check_activity_for_prs(
            db,
            user_id,
            activity.id,
            distance_m,
            &activity.duration,
            start,
        )
        .await
        {
            Ok(prs) => all_new_prs.extend(prs),
            Err(e) => tracing::warn!("PR check failed for activity {}: {e}", activity.id),
        }
    }

    // Update weekly mission progress and detect completions.
    let mut completed_missions = weekly_missions::service::update_progress_after_upload(db, user_id)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!("Weekly mission progress update failed: {e}");
            vec![]
        });

    // Update monthly mission progress and detect completions.
    let completed_monthly = monthly_missions::service::update_progress_after_upload(db, user_id)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!("Monthly mission progress update failed: {e}");
            vec![]
        });
    completed_missions.extend(completed_monthly);

    // Trigger challenge progression for all active challenges of this user.
    // Failure is non-fatal — log and continue so the upload response is unaffected.
    if let Err(e) = crate::challenges::progression::handle(
        db,
        crate::challenges::progression::ProgressionTrigger::ActivitiesUploaded { user_id },
    )
    .await
    {
        tracing::warn!("Challenge progression failed after activity upload: {e}");
    }

    UploadResponse {
        processed: activities.len() as u32,
        xp_earned,
        new_level,
        newly_unlocked_achievements: all_unlocked,
        new_prs: all_new_prs,
        completed_missions,
    }
}

/// Return heatmap grid points for a user, with optional filters.
pub async fn get_heatmap(
    db: &PgPool,
    user_id: Uuid,
    activity_type: Option<String>,
    date_from: Option<NaiveDate>,
    date_to: Option<NaiveDate>,
) -> Result<Vec<HeatmapPoint>, AppError> {
    repository::find_heatmap_points(db, user_id, activity_type, date_from, date_to).await
}
