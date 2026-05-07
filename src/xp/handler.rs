use actix_web::{get, web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

use super::service;

#[utoipa::path(
    get,
    path = "/users/{user_id}/xp",
    params(
        ("user_id" = String, Path, description = "User ID (UUID v4)")
    ),
    responses(
        (status = 200, description = "XP summary for user", body = super::models::UserXpResponse, content_type = "application/json"),
        (status = 400, description = "Invalid UUID"),
        (status = 500, description = "Internal Server Error")
    )
)]
#[get("/users/{user_id}/xp")]
pub async fn get_user_xp(
    path: web::Path<String>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let user_id = Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid UUID".into()))?;

    let summary = service::get_user_xp_summary(db.get_ref(), user_id).await?;
    Ok(HttpResponse::Ok().json(summary))
}
