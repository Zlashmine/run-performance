#[cfg(test)]
mod tests {

    use activity_api::activities::{
        handlers::{get_activities, get_heatmap, get_trackpoints},
        models::{HeatmapPoint, TrackPoint},
    };
    use actix_web::{test, App};
    use sqlx::PgPool;
    use uuid::Uuid;

    async fn setup_db() -> PgPool {
        dotenv::from_filename(".env.test").ok();
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for tests");
        PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    #[actix_web::test]
    async fn test_get_trackpoints_empty() {
        let db = setup_db().await;
        let activity_id = Uuid::new_v4();

        let app = test::init_service(
            App::new()
                .app_data(actix_web::web::Data::new(db.clone()))
                .service(get_trackpoints),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/trackpoints/{}", activity_id))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let trackpoints: Vec<TrackPoint> = test::read_body_json(resp).await;
        assert!(trackpoints.is_empty());
    }

    #[actix_web::test]
    async fn test_get_activities_invalid_uuid() {
        let db = setup_db().await;

        let app = test::init_service(
            App::new()
                .app_data(actix_web::web::Data::new(db.clone()))
                .service(get_activities),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/users/invalid-uuid/activities")
            .to_request();

        let resp = test::call_service(&app, req).await;
        // Invalid UUID → AppError::BadRequest → 400
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    async fn test_get_heatmap_empty() {
        // An unknown user_id should return 200 with an empty array, not 404.
        let db = setup_db().await;
        let unknown_user = Uuid::new_v4();

        let app = test::init_service(
            App::new()
                .app_data(actix_web::web::Data::new(db.clone()))
                .service(get_heatmap),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!("/users/{}/heatmap", unknown_user))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        let points: Vec<HeatmapPoint> = test::read_body_json(resp).await;
        assert!(points.is_empty());
    }

    #[actix_web::test]
    async fn test_get_heatmap_invalid_date_range() {
        // date_from > date_to must return 400 Bad Request.
        let db = setup_db().await;
        let user_id = Uuid::new_v4();

        let app = test::init_service(
            App::new()
                .app_data(actix_web::web::Data::new(db.clone()))
                .service(get_heatmap),
        )
        .await;

        let req = test::TestRequest::get()
            .uri(&format!(
                "/users/{}/heatmap?date_from=2025-12-31&date_to=2025-01-01",
                user_id
            ))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }
}
