pub mod models;
pub mod utils;

use std::collections::HashMap;

use actix_web::{get, post, web, HttpResponse, Responder};
use models::{Activity, ActivityWithTrackPoint, NewActivity, TrackPoint};
use sqlx::PgPool;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/activities",
    responses(
        (status = 200, description = "List all activities", body = [ActivityWithTrackPoint])
    )
)]
#[get("/activities")]
pub async fn get_activities(db: web::Data<PgPool>) -> impl Responder {
    let rows = sqlx::query_as::<_, ActivityWithTrackPoint>(
        "SELECT a.*, tp.id AS trackpoint_id, tp.lat as latitude, tp.lon as longitude, tp.elevation, tp.time, tp.activity_id  FROM activities a INNER JOIN trackpoints tp ON a.id = tp.activity_id ORDER BY a.date DESC",
    )
    .fetch_all(db.get_ref())
    .await;

    if rows.is_err() {
        eprintln!(
            "Error fetching activities: {}",
            rows.as_ref().err().unwrap()
        );

        return HttpResponse::InternalServerError().finish();
    }

    let rows = rows.unwrap();
    let mut grouped: HashMap<Uuid, Activity> = HashMap::new();

    for row in rows {
        let entry = grouped.entry(row.id).or_insert_with(|| Activity {
            id: row.id,
            date: row.date,
            name: row.name,
            activity_type: row.activity_type,
            distance: row.distance,
            duration: row.duration,
            average_pace: row.average_pace,
            average_speed: row.average_speed,
            calories: row.calories,
            climb: row.climb,
            gps_file: row.gps_file,
            track_points: Some(vec![]),
        });

        entry.track_points.as_mut().unwrap().push(TrackPoint {
            id: row.trackpoint_id,
            activity_id: row.activity_id,
            latitude: row.latitude,
            longitude: row.longitude,
            elevation: row.elevation,
            time: row.time.to_string(),
        });
    }

    let activities: Vec<Activity> = grouped.into_values().collect();

    HttpResponse::Ok().json(activities)
}

#[utoipa::path(
    post,
    path = "/activities",
    request_body(content = Vec<NewActivity>, description = "New activities to insert", content_type = "application/json"),
    responses(
        (status = 201, description = "Activities created")
    )
)]
#[post("/activities")]
pub async fn post_activities(
    db: web::Data<PgPool>,
    items: web::Json<Vec<NewActivity>>,
) -> impl Responder {
    for item in items.into_inner() {
        let _ = sqlx::query("INSERT INTO activities (id, name, time) VALUES ($1, $2, $3)")
            .bind(Uuid::new_v4())
            .bind(item.name)
            .bind(item.time)
            .execute(db.get_ref())
            .await;
    }

    HttpResponse::Created().finish()
}
