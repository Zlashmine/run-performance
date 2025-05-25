pub mod models;
pub mod utils;

use actix_web::{get, post, web, HttpResponse, Responder};
use models::{ActivitiesResponse, Activity, ActivityDetailResponse, NewActivity, TrackPoint};
use sqlx::PgPool;
use uuid::Uuid;

use crate::aggregate::aggretate_activities;

#[utoipa::path(
    get,
    path = "/users/{user_id}/activities",
    params(
        ("user_id" = String, description = "User ID", example = "123e4567-e89b-12d3-a456-426614174000")
    ),
    responses(
        (status = 200, description = "List all activities", body = ActivitiesResponse, content_type = "application/json"),
        (status = 400, description = "Bad Request"),
        (status = 500, description = "Internal Server Error")
    )
)]
#[get("/users/{user_id}/activities")]
pub async fn get_activities(path: web::Path<String>, db: web::Data<PgPool>) -> impl Responder {
    let user_id = path.into_inner();

    if user_id.is_empty() || Uuid::parse_str(&user_id).is_err() {
        return HttpResponse::BadRequest().finish();
    }
    let user_id = Uuid::parse_str(&user_id).unwrap();

    let rows = sqlx::query_as::<_, Activity>(
        "SELECT a.* FROM activities a WHERE a.user_id = $1 ORDER BY a.date DESC",
    )
    .bind(user_id)
    .fetch_all(db.get_ref())
    .await;

    let activities = match rows {
        Ok(list) => list,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let aggregation = aggretate_activities(&activities);

    HttpResponse::Ok().json(ActivitiesResponse {
        activities,
        aggregation: Some(aggregation),
    })
}

#[utoipa::path(
    get,
    path = "/activities/{activity_id}",
    params(
        ("activity_id" = String, description = "Activity ID", example = "123e4567-e89b-12d3-a456-426614174000")
    ),
    responses(
        (status = 200, description = "Activity detail", body = ActivityDetailResponse, content_type = "application/json"),
        (status = 400, description = "Bad Request"),
        (status = 404, description = "Not Found"),
        (status = 500, description = "Internal Server Error")
    )
)]
#[get("/activities/{activity_id}")]
pub async fn get_activity_detail(path: web::Path<String>, db: web::Data<PgPool>) -> impl Responder {
    let id_str = path.into_inner();

    let activity_id = match Uuid::parse_str(&id_str) {
        Ok(id) => id,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Invalid UUID: {}", e)),
    };

    let activity_opt = sqlx::query_as::<_, Activity>("SELECT * FROM activities WHERE id = $1")
        .bind(activity_id)
        .fetch_optional(db.get_ref())
        .await;

    let activity = match activity_opt {
        Ok(Some(a)) => a,
        Ok(None) => return HttpResponse::NotFound().finish(),
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!("Database error: {}", e))
        }
    };

    let tps = sqlx::query_as::<_, TrackPoint>(
        "SELECT *, tp.lat AS latitude, tp.lon AS longitude FROM trackpoints tp WHERE tp.activity_id = $1 ORDER BY time DESC",
    )
    .bind(activity_id)
    .fetch_all(db.get_ref())
    .await;

    let track_points = match tps {
        Ok(v) => v,
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!("Database error: {}", e))
        }
    };

    HttpResponse::Ok().json(ActivityDetailResponse {
        activity,
        track_points,
    })
}

#[utoipa::path(
    get,
    path = "/trackpoints/{activity_id}",
    params(
        ("activity_id" = String, description = "Activity ID", example = "123e4567-e89b-12d3-a456-426614174000")
    ),
    responses(
        (status = 200, description = "List all track points", body = Vec<TrackPoint>, content_type = "application/json"),
        (status = 400, description = "Bad Request"),
        (status = 500, description = "Internal Server Error")
    )
)]
#[get("/trackpoints/{activity_id}")]
pub async fn get_trackpoints(path: web::Path<String>, db: web::Data<PgPool>) -> impl Responder {
    let activity_id = path.into_inner();
    if activity_id.is_empty() || Uuid::parse_str(&activity_id).is_err() {
        return HttpResponse::BadRequest().finish();
    }
    let activity_id = Uuid::parse_str(&activity_id).unwrap();

    let rows = sqlx::query_as::<_, TrackPoint>(
        "SELECT tp.id, tp.activity_id, tp.id AS trackpoint_id, tp.lat AS latitude, tp.lon AS longitude, tp.elevation, tp.time \
         FROM trackpoints tp WHERE tp.activity_id = $1 ORDER BY tp.time DESC"
    )
    .bind(activity_id)
    .fetch_all(db.get_ref())
    .await;

    let track_points = match rows {
        Ok(list) => list,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    HttpResponse::Ok().json(track_points)
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
