/// SQL layer for the activities domain.
///
/// All queries live here — no SQL in services or handlers.
use std::collections::HashMap;

use sqlx::{PgPool, QueryBuilder};
use tracing::{error, info};
use uuid::Uuid;

use crate::error::AppError;

use super::models::{Activity, TrackPoint};

pub async fn find_all_by_user(db: &PgPool, user_id: Uuid) -> Result<Vec<Activity>, AppError> {
    sqlx::query_as::<_, Activity>("SELECT * FROM activities WHERE user_id = $1 ORDER BY date DESC")
        .bind(user_id)
        .fetch_all(db)
        .await
        .map_err(AppError::from)
}

pub async fn find_by_id(db: &PgPool, activity_id: Uuid) -> Result<Option<Activity>, AppError> {
    sqlx::query_as::<_, Activity>("SELECT * FROM activities WHERE id = $1")
        .bind(activity_id)
        .fetch_optional(db)
        .await
        .map_err(AppError::from)
}

pub async fn find_trackpoints(db: &PgPool, activity_id: Uuid) -> Result<Vec<TrackPoint>, AppError> {
    sqlx::query_as::<_, TrackPoint>(
        "SELECT id, activity_id, lat AS latitude, lon AS longitude, elevation, time \
         FROM trackpoints WHERE activity_id = $1 ORDER BY time ASC",
    )
    .bind(activity_id)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}

/// Bulk-insert activities, ignoring rows that violate the unique constraint
/// `uq_activities_user_date` (same user + same date).
pub async fn insert_activities(db: &PgPool, activities: &[Activity]) {
    if activities.is_empty() {
        return;
    }

    let mut builder = QueryBuilder::new(
        "INSERT INTO activities \
         (id, user_id, date, name, activity_type, distance, duration, \
          average_pace, average_speed, calories, climb, gps_file) ",
    );

    builder.push_values(activities, |mut b, a| {
        b.push_bind(a.id)
            .push_bind(a.user_id)
            .push_bind(a.date)
            .push_bind(&a.name)
            .push_bind(&a.activity_type)
            .push_bind(a.distance)
            .push_bind(&a.duration)
            .push_bind(a.average_pace)
            .push_bind(a.average_speed)
            .push_bind(a.calories)
            .push_bind(a.climb)
            .push_bind(&a.gps_file);
    });
    builder.push(" ON CONFLICT (user_id, date) DO NOTHING");

    match builder.build().execute(db).await {
        Ok(r) => info!(
            "Inserted {} activities (rest were duplicates)",
            r.rows_affected()
        ),
        Err(e) => error!("Error inserting activities: {}", e),
    }
}

/// Bulk-insert track points for multiple activities.
///
/// **D5 fix**: the existing-ID check is scoped to the relevant `activity_id`s
/// only, preventing a full-table scan on every upload.
///
/// **Q17 fix**: uses `ON CONFLICT (activity_id, time) DO NOTHING` so the
/// random UUID approach doesn't cause duplicate-row confusion.
pub async fn insert_trackpoints(db: &PgPool, trackpoints_map: &HashMap<Uuid, Vec<TrackPoint>>) {
    if trackpoints_map.is_empty() {
        return;
    }

    // Collect all activity IDs so the scoped NOT IN check is tight.
    let activity_ids: Vec<Uuid> = trackpoints_map.keys().copied().collect();

    // Load only the existing trackpoint timestamps for the relevant activities.
    // Result is a set of (activity_id, time) pairs we can skip.
    let existing: std::collections::HashSet<(Uuid, chrono::DateTime<chrono::Utc>)> =
        sqlx::query_as::<_, (Uuid, chrono::DateTime<chrono::Utc>)>(
            "SELECT activity_id, time FROM trackpoints WHERE activity_id = ANY($1)",
        )
        .bind(&activity_ids)
        .fetch_all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .collect();

    let mut tx = match db.begin().await {
        Ok(t) => t,
        Err(e) => {
            error!("Could not begin transaction: {}", e);
            return;
        }
    };

    let mut total_inserted: u64 = 0;

    for (activity_id, trackpoints) in trackpoints_map {
        let new_tps: Vec<&TrackPoint> = trackpoints
            .iter()
            .filter(|tp| !existing.contains(&(*activity_id, tp.time)))
            .collect();

        if new_tps.is_empty() {
            continue;
        }

        let mut builder = QueryBuilder::new(
            "INSERT INTO trackpoints (id, activity_id, lat, lon, elevation, time) ",
        );

        builder.push_values(&new_tps, |mut b, tp| {
            b.push_bind(tp.id.unwrap_or_else(uuid::Uuid::new_v4))
                .push_bind(tp.activity_id)
                .push_bind(tp.latitude)
                .push_bind(tp.longitude)
                .push_bind(tp.elevation)
                .push_bind(tp.time);
        });
        builder.push(" ON CONFLICT (activity_id, time) DO NOTHING");

        match builder.build().execute(&mut *tx).await {
            Ok(r) => total_inserted += r.rows_affected(),
            Err(e) => {
                error!(
                    "Error inserting trackpoints for activity {}: {}",
                    activity_id, e
                );
                let _ = tx.rollback().await;
                return;
            }
        }
    }

    match tx.commit().await {
        Ok(_) => info!("Inserted {} trackpoints", total_inserted),
        Err(e) => error!("Error committing trackpoints: {}", e),
    }
}
