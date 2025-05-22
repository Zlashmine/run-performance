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
pub async fn create_user(payload: web::Json<CreateUser>, db: web::Data<PgPool>) -> impl Responder {
    if !ValidateEmail::validate_email(&payload.email) {
        return HttpResponse::BadRequest().body("Invalid email format");
    }

    let existing_user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(&payload.email)
        .fetch_optional(db.get_ref())
        .await;

    if let Ok(Some(_)) = existing_user {
        return HttpResponse::Conflict().body("User with this email already exists");
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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use sqlx::PgPool;
    use uuid::Uuid;

    async fn setup_db() -> PgPool {
        dotenv::from_filename(".env.test").ok(); // load test-specific env file
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for tests");
        PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    #[actix_web::test]
    async fn test_create_and_get_user() {
        let db = setup_db().await;

        let _ = sqlx::query("DELETE FROM users WHERE email = $1")
            .bind("test@example.com")
            .execute(&db)
            .await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(db.clone()))
                .service(create_user)
                .service(get_user),
        )
        .await;

        let create_payload = CreateUser {
            google_id: "test-google-id".to_string(),
            email: "test@example.com".to_string(),
        };

        let req = test::TestRequest::post()
            .uri("/users")
            .set_json(&create_payload)
            .to_request();

        let resp: User = test::call_and_read_body_json(&app, req).await;
        assert_eq!(resp.email, "test@example.com");

        let user_id = resp.id;

        let req = test::TestRequest::get()
            .uri(&format!("/users/{}", user_id))
            .to_request();

        let fetched: User = test::call_and_read_body_json(&app, req).await;
        assert_eq!(fetched.id, user_id);
        assert_eq!(fetched.email, "test@example.com");
    }

    #[actix_web::test]
    async fn test_create_user_with_invalid_email() {
        let db = setup_db().await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(db.clone()))
                .service(create_user),
        )
        .await;

        let create_payload = CreateUser {
            google_id: "test-google-id".to_string(),
            email: "invalid-email".to_string(),
        };

        let req = test::TestRequest::post()
            .uri("/users")
            .set_json(&create_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    async fn test_get_user_not_found() {
        let db = setup_db().await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(db.clone()))
                .service(get_user),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/users/{}", Uuid::new_v4()))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 404);
    }
}
