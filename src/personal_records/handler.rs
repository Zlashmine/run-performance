use actix_web::{get, web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

use super::{models::PersonalRecordsResponse, service};

#[utoipa::path(
    get,
    path = "/users/{user_id}/personal_records",
    tag = "personal_records",
    params(
        ("user_id" = Uuid, Path, description = "User UUID")
    ),
    responses(
        (status = 200, description = "All 5 PR categories with best times", body = PersonalRecordsResponse),
        (status = 500, description = "Internal server error")
    )
)]
#[get("/users/{user_id}/personal_records")]
pub async fn get_user_prs(
    db: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let user_id = path.into_inner();
    let response = service::get_user_prs(db.get_ref(), user_id).await?;
    Ok(HttpResponse::Ok().json(response))
}
