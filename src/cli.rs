use sqlx::PgPool;
use std::path::Path;
use tracing::error;
use tracing::info;

use crate::activities::models::Activity;
use crate::activities::models::TrackPoint;
use crate::activities::utils::get_activites_from_rows;
use crate::activities::utils::get_activities_ids;
use crate::file_utils::get_rows_in_file;

pub async fn run_cli(folder: &str, db_pool: PgPool) {
    let file_path = format!("{}/cardioActivities.csv", folder);
    let rows = match get_rows_in_file(file_path) {
        Ok(rows) => rows,
        Err(e) => {
            error!("Failed to read rows from file: {}", e);
            return;
        }
    };
    let activities: Vec<Activity> = get_activites_from_rows(rows).await;

    if activities.is_empty() {
        error!("No activities found in the file.");
        return;
    }

    handle_activities(&db_pool, &activities).await;
    handle_trackpoints(folder, &db_pool, &activities).await;
}

async fn handle_activities(db_pool: &PgPool, activities: &[Activity]) {
    use sqlx::QueryBuilder;

    let db_ids = get_activities_ids(db_pool.clone()).await;
    let mut tx = db_pool.begin().await.unwrap();

    let new_activities: Vec<&Activity> = activities
        .iter()
        .filter(|a| !db_ids.contains(&a.id))
        .collect();

    let new_activities_count = new_activities.len();

    let mut builder = QueryBuilder::new(
        "INSERT INTO activities (id, date, name, activity_type, distance, duration, average_pace, average_speed, calories, climb, gps_file) "
    );

    builder.push_values(new_activities, |mut b, activity| {
        b.push_bind(activity.id)
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
        Ok(_) => {
            info!("Inserted {} activities", new_activities_count);
        }
        Err(e) => {
            error!("Error inserting activities: {}", e);
            tx.rollback().await.unwrap();
            return;
        }
    }

    info!(
        "Skipped {} activities",
        activities.len() - new_activities_count
    );

    match tx.commit().await {
        Ok(_) => info!("Activity transactions committed successfully"),
        Err(e) => {
            error!("Error committing transaction: {}", e);
        }
    }
}

async fn handle_trackpoints(folder: &str, db_pool: &PgPool, activities: &[Activity]) {
    use sqlx::QueryBuilder;

    let mut tx = db_pool.begin().await.unwrap();
    let mut inserted_count = 0;

    for activity in activities.iter() {
        let id = activity.id;
        let gps_file = format!("{}/{}", folder, activity.gps_file.clone());
        let file_name = Path::new(&gps_file).to_str().unwrap();

        if !file_name.ends_with(".gpx") || !Path::new(file_name).exists() {
            continue;
        }

        match TrackPoint::from_gpx_file(file_name, &id).await {
            Ok(tracks) => {
                let tracks_count = tracks.len();

                if tracks_count == 0 {
                    continue;
                }

                let mut builder = QueryBuilder::new(
                    "INSERT INTO trackpoints (id, activity_id, lat, lon, elevation, time) ",
                );

                builder.push_values(tracks, |mut b, track| {
                    b.push_bind(track.id.unwrap())
                        .push_bind(track.activity_id)
                        .push_bind(track.latitude)
                        .push_bind(track.longitude)
                        .push_bind(track.elevation)
                        .push_bind(track.time);
                });

                match builder.build().execute(&mut *tx).await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Error inserting trackpoints for activity {}: {}", id, e);
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
            Err(e) => error!("Error reading GPX file: {}. Error: {}", file_name, e),
        }
    }

    if inserted_count > 0 {
        match tx.commit().await {
            Ok(_) => info!("Final commit of remaining {} trackpoints.", inserted_count),
            Err(e) => error!("Error committing transaction: {}", e),
        }
    }
}
