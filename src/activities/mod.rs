pub mod models;
pub mod utils;

use actix_web::{get, post, web, HttpResponse, Responder};
use models::{
    ActivitiesResponse, Activity, ActivityDetailResponse, NewActivity, TrackPoint, UploadForm,
};
use sqlx::PgPool;
use uuid::Uuid;

use actix_multipart::Multipart;
use futures_util::stream::StreamExt as _;
use sanitize_filename::sanitize;

use crate::aggregate::aggretate_activities;

use crate::activities::utils::{insert_activities, insert_trackpoints};
use std::collections::HashMap;

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

    let (aggregation, time_aggregations) = aggretate_activities(&activities);

    HttpResponse::Ok().json(ActivitiesResponse {
        activities,
        aggregation: Some(aggregation),
        time_aggregations: Some(time_aggregations),
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

#[utoipa::path(
    post,
    path = "/upload",
    request_body(content = UploadForm, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Upload successful")
    )
)]
#[post("/upload")]
pub async fn upload_files(
    mut payload: Multipart,
    db: web::Data<PgPool>,
    query: web::Query<HashMap<String, String>>,
) -> impl Responder {
    let user_id_str = match query.get("user_id") {
        Some(val) => val,
        None => return HttpResponse::BadRequest().body("Missing user_id query parameter"),
    };

    let user_id = match Uuid::parse_str(user_id_str) {
        Ok(uuid) => uuid,
        Err(_) => return HttpResponse::BadRequest().body("Invalid UUID format"),
    };

    let mut gpx_count = 0;

    println!("Processing uploaded files...");

    let mut gpx_files: std::collections::HashMap<String, Vec<u8>> =
        std::collections::HashMap::new();

    let mut activities_from_file = Vec::new();

    while let Some(item) = payload.next().await {
        match item {
            Ok(mut field) => {
                let content_disposition = field.content_disposition();
                let filename = content_disposition.unwrap().get_filename().map(sanitize);

                if let Some(name) = filename {
                    let mut content = Vec::new();

                    while let Some(chunk) = field.next().await {
                        match chunk {
                            Ok(bytes) => content.extend(bytes),
                            Err(_) => {
                                println!("Error reading chunk from multipart stream");
                                return HttpResponse::InternalServerError().body("Stream error");
                            }
                        }
                    }

                    if name.to_lowercase().ends_with(".gpx") {
                        gpx_count += 1;
                        gpx_files.insert(name.clone(), content);
                    } else if name.to_lowercase() == "cardioactivities.csv" {
                        match std::str::from_utf8(&content) {
                            Ok(text) => {
                                activities_from_file = utils::get_activites_from_rows(
                                    text.lines().map(|line| line.to_string()).collect(),
                                    user_id,
                                )
                                .await;

                                println!(
                                    "cardioActivities.csv activities_from_file:\n{}",
                                    activities_from_file.len()
                                );
                            }
                            Err(_) => {
                                println!("Invalid UTF-8 in cardioActivities.csv");
                                return HttpResponse::InternalServerError()
                                    .body("Invalid UTF-8 in cardioActivities.csv");
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("Error processing multipart item: {}", e);
                return HttpResponse::InternalServerError().body("Multipart error");
            }
        }
    }

    // Add parsed activities to DB
    insert_activities(&db, &activities_from_file, Some(user_id)).await;

    // Build map from activity ID to trackpoints
    let mut trackpoints_map = HashMap::new();
    for activity in &activities_from_file {
        if let Some(gpx_data) = gpx_files.get(&activity.gps_file) {
            match TrackPoint::from_gpx_data(gpx_data, &activity.id).await {
                Ok(tracks) => {
                    trackpoints_map.insert(activity.id, tracks);
                }
                Err(e) => {
                    println!("Failed to parse gpx for {}: {}", activity.gps_file, e);
                }
            }
        }
    }

    insert_trackpoints(&db, &trackpoints_map).await;

    HttpResponse::Ok().body(format!("Received {} .gpx file(s).", gpx_count))
}
