#[cfg(test)]
mod tests {
    use activity_api::users::{
        create_user, get_user,
        models::{CreateUser, User},
    };
    use actix_web::{test, web, App};
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
