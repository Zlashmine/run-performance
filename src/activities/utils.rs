use crate::activities::models::{Activity, TrackPoint};
use sqlx::QueryBuilder;
use std::collections::HashSet;
use tracing::{error, info};

use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;

pub async fn get_activites_from_rows(rows: Vec<String>, user_id: Uuid) -> Vec<Activity> {
    let mut activities: Vec<Activity> = Vec::new();

    for row in rows {
        match Activity::from_csv_row(row.as_str(), user_id) {
            Ok(activity) => activities.push(activity),
            Err(e) => {
                eprintln!("Error parsing row: {}. Error: {}", row, e);
                continue;
            }
        }
    }

    activities
}

pub async fn insert_activities(db_pool: &PgPool, activities: &[Activity], user_id: Option<Uuid>) {
    if user_id.is_none() {
        error!("insert_activities called without user_id.");
        return;
    }
    let user_id = user_id.unwrap();

    let existing_dates: HashSet<chrono::NaiveDateTime> =
        sqlx::query("SELECT date FROM activities WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(db_pool)
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.get("date"))
            .collect();

    let mut tx = db_pool.begin().await.unwrap();

    let new_activities: Vec<&Activity> = activities
        .iter()
        .filter(|a| !existing_dates.contains(&a.date))
        .collect();

    if new_activities.is_empty() {
        info!("No new activities to insert (all duplicates).");
        return;
    }

    let new_activities_count = new_activities.len();

    let mut builder = QueryBuilder::new(
        "INSERT INTO activities (id, user_id, date, name, activity_type, distance, duration, average_pace, average_speed, calories, climb, gps_file) ",
    );

    builder.push_values(new_activities, |mut b, activity| {
        b.push_bind(activity.id)
            .push_bind(activity.user_id)
            .push_bind(activity.date)
            .push_bind(&activity.name)
            .push_bind(&activity.activity_type)
            .push_bind(activity.distance)
            .push_bind(&activity.duration)
            .push_bind(activity.average_pace)
            .push_bind(activity.average_speed)
            .push_bind(activity.calories)
            .push_bind(activity.climb)
            .push_bind(&activity.gps_file);
    });

    match builder.build().execute(&mut *tx).await {
        Ok(_) => info!("Inserted {} activities", new_activities_count),
        Err(e) => {
            error!("Error inserting activities: {}", e);
            tx.rollback().await.unwrap();
            return;
        }
    }

    info!(
        "Skipped {} activities (duplicate dates)",
        activities.len() - new_activities_count
    );

    match tx.commit().await {
        Ok(_) => info!("Activity transactions committed successfully"),
        Err(e) => {
            error!("Error committing transaction: {}", e);
        }
    }
}

use std::collections::HashMap;

pub async fn insert_trackpoints(
    db_pool: &PgPool,
    trackpoints_map: &HashMap<Uuid, Vec<TrackPoint>>,
) {
    let existing_ids: HashSet<Uuid> = sqlx::query("SELECT id FROM trackpoints")
        .fetch_all(db_pool)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.get("id"))
        .collect();

    let mut tx = db_pool.begin().await.unwrap();
    let mut inserted_count = 0;

    for (activity_id, trackpoints) in trackpoints_map {
        let new_trackpoints: Vec<&TrackPoint> = trackpoints
            .iter()
            .filter(|tp| tp.id.is_some_and(|id| !existing_ids.contains(&id)))
            .collect();

        if new_trackpoints.is_empty() {
            continue;
        }

        let mut builder = QueryBuilder::new(
            "INSERT INTO trackpoints (id, activity_id, lat, lon, elevation, time) ",
        );

        builder.push_values(new_trackpoints, |mut b, track| {
            b.push_bind(track.id.unwrap())
                .push_bind(track.activity_id)
                .push_bind(&track.latitude)
                .push_bind(&track.longitude)
                .push_bind(track.elevation)
                .push_bind(&track.time);
        });

        match builder.build().execute(&mut *tx).await {
            Ok(_) => {}
            Err(e) => {
                error!(
                    "Error inserting trackpoints for activity {}: {}",
                    activity_id, e
                );
                tx.rollback().await.unwrap();
                return;
            }
        }

        inserted_count += 1;

        if inserted_count >= 50 {
            match tx.commit().await {
                Ok(_) => info!("Committed 50 trackpoints."),
                Err(e) => {
                    error!("Error committing transaction: {}", e);
                    return;
                }
            }

            tx = db_pool.begin().await.unwrap();
            inserted_count = 0;
        }
    }

    if inserted_count > 0 {
        match tx.commit().await {
            Ok(_) => info!("Final commit of remaining {} trackpoints.", inserted_count),
            Err(e) => error!("Error committing transaction: {}", e),
        }
    }
}
