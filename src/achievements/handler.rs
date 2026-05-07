use actix_web::{get, web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

use super::{models::AchievementWithStatus, service};

#[utoipa::path(
    get,
    path = "/users/{user_id}/achievements",
    tag = "achievements",
    params(
        ("user_id" = Uuid, Path, description = "User UUID")
    ),
    responses(
        (status = 200, description = "List of achievements with unlock status", body = Vec<AchievementWithStatus>),
        (status = 500, description = "Internal server error")
    )
)]
#[get("/users/{user_id}/achievements")]
pub async fn get_user_achievements(
    db: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let user_id = path.into_inner();
    let achievements = service::get_user_achievements(db.get_ref(), user_id).await?;
    Ok(HttpResponse::Ok().json(achievements))
}
