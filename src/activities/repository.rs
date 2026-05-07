/// SQL layer for the activities domain.
///
/// All queries live here — no SQL in services or handlers.
use std::collections::HashMap;

use chrono::NaiveDate;
use sqlx::{PgPool, QueryBuilder};
use tracing::{error, info};
use uuid::Uuid;

use crate::sync::normalized::NormalizedTrackPoint;

use crate::error::AppError;

use super::models::{Activity, HeatmapPoint, TrackPoint};

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

/// Fetch multiple activities by their IDs in a single round-trip.
/// Returns a map of `activity_id → Activity` for O(1) lookup.
pub async fn find_activities_by_ids(
    db: &PgPool,
    ids: &[Uuid],
) -> Result<HashMap<Uuid, Activity>, AppError> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = sqlx::query_as::<_, Activity>("SELECT * FROM activities WHERE id = ANY($1)")
        .bind(ids)
        .fetch_all(db)
        .await
        .map_err(AppError::from)?;
    Ok(rows.into_iter().map(|a| (a.id, a)).collect())
}

/// Fetch all activities for a user starting from `from` (inclusive),
/// optionally capped at `until` (inclusive). Results are sorted by date
/// ascending so the progression engine can iterate chronologically.
pub async fn find_activities_by_user_from(
    db: &PgPool,
    user_id: Uuid,
    from: chrono::DateTime<chrono::Utc>,
    until: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<Vec<Activity>, AppError> {
    sqlx::query_as::<_, Activity>(
        "SELECT * FROM activities
         WHERE user_id = $1
           AND date >= $2
           AND ($3::timestamptz IS NULL OR date <= $3)
         ORDER BY date ASC",
    )
    .bind(user_id)
    .bind(from.naive_utc())
    .bind(until.map(|u| u.naive_utc()))
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}

pub async fn find_trackpoints(db: &PgPool, activity_id: Uuid) -> Result<Vec<TrackPoint>, AppError> {
    sqlx::query_as::<_, TrackPoint>(
        "SELECT id, activity_id, lat AS latitude, lon AS longitude, elevation, time, speed \
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
          average_pace, average_speed, calories, climb, gps_file, source, external_id) ",
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
            .push_bind(&a.gps_file)
            .push_bind(&a.source)
            .push_bind(&a.external_id);
    });
    // Runkeeper rows: dedup by (user_id, date) — legacy behaviour preserved.
    // Strava rows: dedup by the partial unique index on (user_id, source, external_id).
    builder.push(" ON CONFLICT (user_id, date) DO NOTHING");

    match builder.build().execute(db).await {
        Ok(r) => info!(
            "Inserted {} activities (rest were duplicates)",
            r.rows_affected()
        ),
        Err(e) => error!("Error inserting activities: {}", e),
    }
}

/// Insert a batch of activities coming from a remote source (e.g. Strava).
///
/// Deduplication is handled by the `uq_activities_external_source` partial
/// unique index: `(user_id, source, external_id) WHERE external_id IS NOT NULL`.
/// Re-syncing the same activities is therefore fully idempotent.
///
/// Returns the IDs of activities that were actually inserted (not duplicates),
/// so the caller can run the XP/achievement/PR pipelines only on new rows.
pub async fn insert_activities_from_source(
    db: &PgPool,
    user_id: Uuid,
    activities: &[crate::sync::normalized::NormalizedActivity],
) -> Vec<Uuid> {
    if activities.is_empty() {
        return vec![];
    }

    let mut inserted_ids = Vec::new();

    for a in activities {
        let new_id = Uuid::new_v4();
        let result = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO activities
                (id, user_id, date, name, activity_type, distance, duration,
                 average_pace, average_speed, calories, climb, gps_file,
                 source, external_id)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
            ON CONFLICT DO NOTHING
            RETURNING id
            "#,
        )
        .bind(new_id)
        .bind(user_id)
        .bind(a.date)
        .bind(&a.name)
        .bind(&a.activity_type)
        .bind(a.distance)
        .bind(&a.duration)
        .bind(a.average_pace)
        .bind(a.average_speed)
        .bind(a.calories)
        .bind(a.climb)
        .bind(&a.gps_file)
        .bind(&a.source)
        .bind(&a.external_id)
        .fetch_optional(db)
        .await;

        match result {
            Ok(Some(id)) => inserted_ids.push(id),
            Ok(None) => {} // duplicate — skip silently
            Err(e) => error!("Error inserting activity (source={}, ext_id={:?}): {}", a.source, a.external_id, e),
        }
    }

    info!(
        "Inserted {}/{} activities from source",
        inserted_ids.len(),
        activities.len()
    );
    inserted_ids
}

/// Delete an activity by its source + external_id pair.
///
/// Used when a Strava webhook delivers an `aspect_type = "delete"` event.
/// Returns `true` if a row was deleted.
pub async fn delete_by_external_id(
    db: &PgPool,
    user_id: Uuid,
    source: &str,
    external_id: &str,
) -> Result<bool, AppError> {
    let result = sqlx::query(
        "DELETE FROM activities WHERE user_id = $1 AND source = $2 AND external_id = $3",
    )
    .bind(user_id)
    .bind(source)
    .bind(external_id)
    .execute(db)
    .await
    .map_err(AppError::from)?;
    Ok(result.rows_affected() > 0)
}

/// Look up the internal UUID of an activity by its source + external_id.
pub async fn find_id_by_external(
    db: &PgPool,
    user_id: Uuid,
    source: &str,
    external_id: &str,
) -> Result<Option<Uuid>, AppError> {
    sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM activities WHERE user_id = $1 AND source = $2 AND external_id = $3",
    )
    .bind(user_id)
    .bind(source)
    .bind(external_id)
    .fetch_optional(db)
    .await
    .map_err(AppError::from)
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
            "INSERT INTO trackpoints (id, activity_id, lat, lon, elevation, time, speed) ",
        );

        builder.push_values(&new_tps, |mut b, tp| {
            b.push_bind(tp.id.unwrap_or_else(uuid::Uuid::new_v4))
                .push_bind(tp.activity_id)
                .push_bind(tp.latitude)
                .push_bind(tp.longitude)
                .push_bind(tp.elevation)
                .push_bind(tp.time)
                .push_bind(tp.speed);
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

/// Return aggregated heatmap grid points for a user.
///
/// Coordinates are rounded to 4 decimal places (~11 m grid cells).
/// All filter parameters are optional; `None` means "no filter".
pub async fn find_heatmap_points(
    db: &PgPool,
    user_id: Uuid,
    activity_type: Option<String>,
    date_from: Option<NaiveDate>,
    date_to: Option<NaiveDate>,
) -> Result<Vec<HeatmapPoint>, AppError> {
    sqlx::query_as::<_, HeatmapPoint>(
        "SELECT ROUND(t.lat::numeric, 4)::float8 AS lat, \
                ROUND(t.lon::numeric, 4)::float8 AS lon, \
                COUNT(*) AS weight \
         FROM   trackpoints t \
         JOIN   activities a ON a.id = t.activity_id \
         WHERE  a.user_id = $1 \
           AND  ($2::text IS NULL OR a.activity_type = $2) \
           AND  ($3::date IS NULL OR a.date::date >= $3) \
           AND  ($4::date IS NULL OR a.date::date <= $4) \
         GROUP  BY ROUND(t.lat::numeric, 4), ROUND(t.lon::numeric, 4)",
    )
    .bind(user_id)
    .bind(activity_type)
    .bind(date_from)
    .bind(date_to)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}

/// Bulk-insert `NormalizedTrackPoint` values for a single activity.
///
/// Uses `ON CONFLICT (activity_id, time) DO NOTHING` so re-syncing is safe.
pub async fn insert_normalized_trackpoints(
    db: &PgPool,
    activity_id: Uuid,
    points: &[NormalizedTrackPoint],
) {
    if points.is_empty() {
        return;
    }

    let mut builder = QueryBuilder::new(
        "INSERT INTO trackpoints (id, activity_id, lat, lon, elevation, time, speed) ",
    );

    builder.push_values(points, |mut b, tp| {
        b.push_bind(Uuid::new_v4())
            .push_bind(activity_id)
            .push_bind(tp.latitude)
            .push_bind(tp.longitude)
            .push_bind(tp.elevation)
            .push_bind(tp.time)
            .push_bind(tp.speed);
    });
    builder.push(" ON CONFLICT (activity_id, time) DO NOTHING");

    match builder.build().execute(db).await {
        Ok(r) => info!("Inserted {} trackpoints for activity {}", r.rows_affected(), activity_id),
        Err(e) => error!("Error inserting trackpoints for activity {}: {}", activity_id, e),
    }
}
