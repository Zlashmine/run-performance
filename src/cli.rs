use sqlx::PgPool;
use sqlx::Postgres;
use sqlx::Transaction;
use std::fs::{self};
use std::path::Path;
use tracing::info;

use crate::activities::models::Activity;
use crate::activities::models::TrackPoint;
use crate::activities::utils::get_activites_from_rows;
use crate::activities::utils::get_activities_ids;

pub async fn run_cli(folder: &str, db_pool: PgPool) {
    let file_path = format!("{}/cardioActivities.csv", folder);

    let rows = get_rows_in_file(file_path);
    let activities = get_activites_from_rows(rows).await;

    if activities.is_empty() {
        eprintln!("No activities found in the file.");
        return;
    }

    let db_ids = get_activities_ids(db_pool.clone()).await;
    let mut tx = db_pool.begin().await.unwrap();

    let mut added_count = 0;
    let mut skipped_count = 0;

    for activity in activities.iter() {
        if db_ids.contains(&activity.id) {
            skipped_count += 1;
            continue;
        }

        let id = activity.id;

        let result = insert_activity(&mut tx, activity.clone()).await;

        if result.is_ok() {
            added_count += 1;
        }

        if let Err(e) = result {
            eprintln!("Error inserting activity: {:?}. Error: {}", id, e);
            tx.rollback().await.unwrap();
            return;
        }
    }

    info!("Added {} activities", added_count);
    info!("Skipped {} activities", skipped_count);

    match tx.commit().await {
        Ok(_) => info!("Activity Transaction committed successfully"),
        Err(e) => {
            eprintln!("Error committing transaction: {}", e);
            return;
        }
    }

    tx = db_pool.begin().await.unwrap();

    info!("Starting to insert trackpoints");

    let mut inserted_count = 0;

    for activity in activities.iter() {
        let id = activity.id;
        let gps_file = format!("{}/{}", folder, activity.gps_file.clone());
        let file_name = Path::new(&gps_file).to_str().unwrap();

        if !file_name.ends_with(".gpx") || !Path::new(file_name).exists() {
            continue;
        }

        match TrackPoint::from_gpx_file(file_name, &id) {
            Ok(tracks) => {
                for track in tracks {
                    match insert_trackpoint(&mut tx, track).await {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("Error inserting trackpoint: {:?}. Error: {}", id, e);
                            tx.rollback().await.unwrap();
                            return;
                        }
                    }
                }

                if inserted_count >= 50 {
                    match tx.commit().await {
                        Ok(_) => info!("Committed 50 trackpoints."),
                        Err(e) => {
                            eprintln!("Error committing transaction: {}", e);
                            return;
                        }
                    }
                    tx = db_pool.begin().await.unwrap();
                    inserted_count = 0;
                }

                inserted_count += 1;
            }
            Err(e) => {
                eprintln!("Error reading GPX file: {}. Error: {}", file_name, e);
                continue;
            }
        }
    }

    if inserted_count > 0 {
        match tx.commit().await {
            Ok(_) => info!("Final commit of remaining {} trackpoints.", inserted_count),
            Err(e) => {
                eprintln!("Error committing transaction: {}", e);
            }
        }
    }
}

fn get_rows_in_file(file: String) -> Vec<&'static str> {
    let path = Path::new(&file);

    if !path.exists() {
        eprintln!("File does not exist: {}", file);
        std::process::exit(1);
    }

    let content = Box::leak(
        fs::read_to_string(path)
            .expect("Failed to read file")
            .into_boxed_str(),
    );

    content.lines().collect()
}

async fn insert_activity(
    tx: &mut Transaction<'_, Postgres>,
    activity: Activity,
) -> Result<(), sqlx::Error> {
    let _result = sqlx::query("INSERT INTO activities (id, date, name, activity_type, distance, duration, average_pace, average_speed, calories, climb, gps_file) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)")
        .bind(activity.id)
        .bind(activity.date)
        .bind(activity.name)
        .bind(activity.activity_type)
        .bind(activity.distance)
        .bind(activity.duration)
        .bind(activity.average_pace)
        .bind(activity.average_speed)
        .bind(activity.calories)
        .bind(activity.climb)
        .bind(activity.gps_file)
        .execute(&mut **tx)
        .await?;

    Ok(())
}

async fn insert_trackpoint(
    tx: &mut Transaction<'_, Postgres>,
    trackpoint: TrackPoint,
) -> Result<(), sqlx::Error> {
    let _result = sqlx::query("INSERT INTO trackpoints (id, activity_id, lat, lon, elevation, time) VALUES ($1, $2, $3, $4, $5, $6)")
        .bind(trackpoint.id.unwrap())
        .bind(trackpoint.activity_id)
        .bind(trackpoint.latitude)
        .bind(trackpoint.longitude)
        .bind(trackpoint.elevation)
        .bind(trackpoint.time)
        .execute(&mut **tx)
        .await?;

    Ok(())
}
