/// HTTP handlers for the activities domain.
///
/// Each handler parses the request, delegates to `service`, and maps results
/// to HTTP responses.  No SQL and no file-parsing logic here.
use std::collections::HashMap;

use actix_multipart::Multipart;
use actix_web::{get, post, web, HttpResponse};
use futures_util::stream::StreamExt as _;
use sanitize_filename::sanitize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

use super::{models::UploadForm, service};

#[utoipa::path(
    get,
    path = "/users/{user_id}/activities",
    params(
        ("user_id" = String, description = "User ID (UUID v4)", example = "123e4567-e89b-12d3-a456-426614174000")
    ),
    responses(
        (status = 200, description = "List of activities with aggregations", body = super::models::ActivitiesResponse, content_type = "application/json"),
        (status = 400, description = "Invalid UUID"),
        (status = 500, description = "Internal Server Error")
    )
)]
#[get("/users/{user_id}/activities")]
pub async fn get_activities(
    path: web::Path<String>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let user_id = Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid UUID".into()))?;

    let result = service::get_activities(db.get_ref(), user_id).await?;
    Ok(HttpResponse::Ok().json(result))
}

#[utoipa::path(
    get,
    path = "/activities/{activity_id}",
    params(
        ("activity_id" = String, description = "Activity ID (UUID v4)", example = "123e4567-e89b-12d3-a456-426614174000")
    ),
    responses(
        (status = 200, description = "Activity detail with GPS track", body = super::models::ActivityDetailResponse, content_type = "application/json"),
        (status = 400, description = "Invalid UUID"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal Server Error")
    )
)]
#[get("/activities/{activity_id}")]
pub async fn get_activity_detail(
    path: web::Path<String>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let activity_id = Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid UUID".into()))?;

    let result = service::get_activity_detail(db.get_ref(), activity_id).await?;
    Ok(HttpResponse::Ok().json(result))
}

#[utoipa::path(
    get,
    path = "/trackpoints/{activity_id}",
    params(
        ("activity_id" = String, description = "Activity ID (UUID v4)", example = "123e4567-e89b-12d3-a456-426614174000")
    ),
    responses(
        (status = 200, description = "Track points for an activity", body = Vec<super::models::TrackPoint>, content_type = "application/json"),
        (status = 400, description = "Invalid UUID"),
        (status = 500, description = "Internal Server Error")
    )
)]
#[get("/trackpoints/{activity_id}")]
pub async fn get_trackpoints(
    path: web::Path<String>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let activity_id = Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid UUID".into()))?;

    let tps = service::get_trackpoints(db.get_ref(), activity_id).await?;
    Ok(HttpResponse::Ok().json(tps))
}

#[utoipa::path(
    post,
    path = "/upload",
    params(
        ("user_id" = String, Query, description = "User ID (UUID v4)")
    ),
    request_body(content = UploadForm, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Upload processed successfully"),
        (status = 400, description = "Bad request (missing/invalid user_id or multipart error)")
    )
)]
#[post("/upload")]
pub async fn upload_files(
    mut payload: Multipart,
    db: web::Data<PgPool>,
    query: web::Query<HashMap<String, String>>,
) -> Result<HttpResponse, AppError> {
    let user_id_str = query
        .get("user_id")
        .ok_or_else(|| AppError::BadRequest("Missing user_id query parameter".into()))?;

    let user_id = Uuid::parse_str(user_id_str)
        .map_err(|_| AppError::BadRequest("Invalid UUID format".into()))?;

    let mut csv_lines: Vec<String> = Vec::new();
    let mut gpx_files: HashMap<String, Vec<u8>> = HashMap::new();

    while let Some(item) = payload.next().await {
        let mut field = item.map_err(|e| {
            tracing::error!("Multipart stream error: {}", e);
            AppError::BadRequest("Multipart stream error".into())
        })?;

        let filename = field
            .content_disposition()
            .and_then(|cd| cd.get_filename().map(sanitize));

        let Some(name) = filename else {
            continue;
        };

        let mut content = Vec::new();
        while let Some(chunk) = field.next().await {
            let bytes = chunk.map_err(|e| {
                tracing::error!("Error reading chunk: {}", e);
                AppError::BadRequest("Error reading upload chunk".into())
            })?;
            content.extend(bytes);
        }

        if name.to_lowercase().ends_with(".gpx") {
            gpx_files.insert(name, content);
        } else if name.to_lowercase() == "cardioactivities.csv" {
            match std::str::from_utf8(&content) {
                Ok(text) => {
                    csv_lines = text.lines().map(|l| l.to_string()).collect();
                }
                Err(_) => {
                    return Err(AppError::BadRequest(
                        "Invalid UTF-8 in cardioActivities.csv".into(),
                    ))
                }
            }
        }
    }

    let gpx_count = service::upload(db.get_ref(), user_id, csv_lines, gpx_files).await;
    Ok(HttpResponse::Ok().body(format!("Processed {} GPX file(s).", gpx_count)))
}
