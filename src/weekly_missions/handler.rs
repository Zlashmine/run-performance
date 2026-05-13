use actix_web::{web, HttpResponse};
use uuid::Uuid;

use crate::error::AppError;
use sqlx::PgPool;

use super::service;

/// Get the current week's missions for a user (lazily generates if needed).
#[utoipa::path(
    get,
    path = "/users/{user_id}/weekly_missions",
    params(("user_id" = Uuid, Path, description = "User ID")),
    responses(
        (status = 200, description = "Weekly missions", body = super::models::WeeklyMissionsResponse),
        (status = 404, description = "User not found"),
    ),
    tag = "missions"
)]
pub async fn get_weekly_missions(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let user_id = path.into_inner();
    let response = service::get_or_generate_missions(&pool, user_id).await?;
    Ok(HttpResponse::Ok().json(response))
}

/// Reroll one mission — replaces it with a new mission of a different type.
#[utoipa::path(
    post,
    path = "/users/{user_id}/weekly_missions/{mission_id}/reroll",
    params(
        ("user_id" = Uuid, Path, description = "User ID"),
        ("mission_id" = Uuid, Path, description = "Mission ID to reroll"),
    ),
    responses(
        (status = 200, description = "New replacement mission", body = super::models::WeeklyMission),
        (status = 400, description = "Reroll already used or invalid"),
        (status = 404, description = "Mission not found"),
    ),
    tag = "missions"
)]
pub async fn reroll_mission(
    pool: web::Data<PgPool>,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse, AppError> {
    let (user_id, mission_id) = path.into_inner();
    let mission = service::reroll_mission(&pool, user_id, mission_id).await?;
    Ok(HttpResponse::Ok().json(mission))
}
