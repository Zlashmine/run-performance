/// HTTP handlers for the users domain.
///
/// Each handler parses the request, delegates to `service`, and maps the
/// result to an HTTP response.  No SQL lives here.
use actix_web::{get, post, web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;
use validator::ValidateEmail;

use crate::error::AppError;

use super::{models::CreateUser, service};

#[utoipa::path(
    get,
    path = "/users/{user_id}",
    params(
        ("user_id" = String, description = "User ID (UUID v4)", example = "123e4567-e89b-12d3-a456-426614174000")
    ),
    responses(
        (status = 200, description = "User found",  body = super::models::User, content_type = "application/json"),
        (status = 400, description = "Invalid UUID"),
        (status = 404, description = "User not found")
    )
)]
#[get("/users/{user_id}")]
pub async fn get_user(
    path: web::Path<String>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let user_id = Uuid::parse_str(&path.into_inner())
        .map_err(|_| AppError::BadRequest("Invalid UUID".into()))?;

    let user = service::get_user(db.get_ref(), user_id).await?;
    Ok(HttpResponse::Ok().json(user))
}

#[utoipa::path(
    post,
    path = "/users",
    request_body = CreateUser,
    responses(
        (status = 201, description = "User created or returned",  body = super::models::User, content_type = "application/json"),
        (status = 400, description = "Validation error")
    )
)]
#[post("/users")]
pub async fn create_user(
    body: web::Json<CreateUser>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, AppError> {
    let payload = body.into_inner();

    if !ValidateEmail::validate_email(&payload.email) {
        return Err(AppError::BadRequest("Invalid email address".into()));
    }

    let user = service::upsert_user(db.get_ref(), &payload).await?;
    Ok(HttpResponse::Created().json(user))
}
