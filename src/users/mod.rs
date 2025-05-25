use actix_web::post;
use actix_web::{get, web, HttpResponse, Responder};
use models::{CreateUser, User};
use sqlx::PgPool;
use uuid::Uuid;
use validator::ValidateEmail;
pub mod models;

#[utoipa::path(
    get,
    path = "/users/:user_id",
    params(
        ("user_id" = String, description = "User ID", example = "123e4567-e89b-12d3-a456-426614174000")
    ),
    responses(
        (status = 200, description = "Get user from path", body = User, content_type = "application/json")
    )
)]
#[get("/users/{user_id}")]
pub async fn get_user(path: web::Path<String>, db: web::Data<PgPool>) -> impl Responder {
    let user_id = path.into_inner();

    if user_id.is_empty() {
        return HttpResponse::BadRequest().finish();
    }

    if Uuid::parse_str(&user_id).is_err() {
        return HttpResponse::BadRequest().finish();
    }

    let user_id = Uuid::parse_str(&user_id).unwrap();

    match sqlx::query_as::<_, User>("SELECT u.* FROM users u WHERE u.id = $1")
        .bind(user_id)
        .fetch_one(db.get_ref())
        .await
    {
        Ok(user) => HttpResponse::Ok().json(user),
        Err(sqlx::Error::RowNotFound) => HttpResponse::NotFound().body("User not found"),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[utoipa::path(
    post,
    path = "/users",
    request_body = CreateUser,
    responses(
        (status = 201, description = "User created", body = User, content_type = "application/json")
    )
)]
#[post("/users")]
pub async fn create_user(p: web::Json<CreateUser>, db: web::Data<PgPool>) -> impl Responder {
    let payload = p.into_inner();

    if !ValidateEmail::validate_email(&payload.email) {
        return HttpResponse::BadRequest().body("Invalid email format");
    }

    let existing_user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(&payload.email)
        .fetch_optional(db.get_ref())
        .await;

    if let Ok(Some(_)) = existing_user {
        return HttpResponse::Ok().json(existing_user.unwrap());
    }

    let new_id = Uuid::new_v4();

    match sqlx::query_as::<_, User>(
        "INSERT INTO users (id, google_id, email, created_at) VALUES ($1, $2, $3, now()) RETURNING *"
    )
    .bind(new_id)
    .bind(&payload.google_id)
    .bind(&payload.email)
    .fetch_one(db.get_ref())
    .await
    {
        Ok(user) => HttpResponse::Created().json(user),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
