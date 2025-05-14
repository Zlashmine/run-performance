use std::collections::HashSet;

use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;

use super::models::Activity;

pub async fn get_activites_from_rows(rows: Vec<String>) -> Vec<Activity> {
    let mut activities: Vec<Activity> = Vec::new();

    for row in rows {
        match Activity::from_csv_row(row.as_str()) {
            Ok(activity) => activities.push(activity),
            Err(e) => {
                eprintln!("Error parsing row: {}. Error: {}", row, e);
                continue;
            }
        }
    }

    activities
}

pub async fn get_activities_ids(db_pool: PgPool) -> HashSet<Uuid> {
    let activities_from_db = sqlx::query("SELECT * FROM activities")
        .fetch_all(&db_pool)
        .await
        .unwrap();

    activities_from_db
        .iter()
        .map(|row| row.get::<Uuid, _>("id"))
        .collect()
}
