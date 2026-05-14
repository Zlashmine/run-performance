use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

use super::{models::CreateGoalRequest, service};/// List all goals for a user (with requirements and current period progress).
#[utoipa::path(
    get,
    path = "/users/{user_id}/goals",
    params(("user_id" = Uuid, Path, description = "User ID")),
    responses(
        (status = 200, description = "Goals list", body = Vec<super::models::UserGoalResponse>),
        (status = 404, description = "User not found"),
    ),
    tag = "goals"
)]
pub async fn list_goals(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let user_id = path.into_inner();
    let goals = service::list_goals(&pool, user_id).await?;
    Ok(HttpResponse::Ok().json(goals))
}

/// Create a new goal for a user (max 3 active slots).
#[utoipa::path(
    post,
    path = "/users/{user_id}/goals",
    params(("user_id" = Uuid, Path, description = "User ID")),
    request_body = CreateGoalRequest,
    responses(
        (status = 201, description = "Goal created", body = super::models::UserGoalResponse),
        (status = 400, description = "Validation error or goal_limit_reached"),
        (status = 404, description = "User not found"),
    ),
    tag = "goals"
)]
pub async fn create_goal(
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    body: web::Json<CreateGoalRequest>,
) -> Result<HttpResponse, AppError> {
    let user_id = path.into_inner();
    let goal = service::create_goal(&pool, user_id, body.into_inner()).await?;
    Ok(HttpResponse::Created().json(goal))
}

/// Delete a goal by ID.
#[utoipa::path(
    delete,
    path = "/users/{user_id}/goals/{goal_id}",
    params(
        ("user_id" = Uuid, Path, description = "User ID"),
        ("goal_id" = Uuid, Path, description = "Goal ID"),
    ),
    responses(
        (status = 204, description = "Goal deleted"),
        (status = 403, description = "Forbidden — not the owner"),
        (status = 404, description = "Goal not found"),
    ),
    tag = "goals"
)]
pub async fn delete_goal(
    pool: web::Data<PgPool>,
    path: web::Path<(Uuid, Uuid)>,
) -> Result<HttpResponse, AppError> {
    let (user_id, goal_id) = path.into_inner();
    service::delete_goal(&pool, goal_id, user_id).await?;
    Ok(HttpResponse::NoContent().finish())
}
