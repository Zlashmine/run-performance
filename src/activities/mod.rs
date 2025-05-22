pub mod models;
pub mod utils;

use actix_web::{get, post, web, HttpResponse, Responder};
use models::{ActivitiesResponse, Activity, NewActivity, TrackPoint};
use sqlx::PgPool;
use uuid::Uuid;

use crate::aggregate::aggretate_activities;

#[utoipa::path(
    get,
    path = "/activities/:user_id",
    params(
        ("user_id" = String, description = "User ID", example = "123e4567-e89b-12d3-a456-426614174000")
    ),
    responses(
        (status = 200, description = "List all activities", body = ActivitiesResponse, content_type = "application/json")
    )
)]
#[get("/activities/{user_id}")]
pub async fn get_activities(path: web::Path<String>, db: web::Data<PgPool>) -> impl Responder {
    let user_id = path.into_inner();

    if user_id.is_empty() {
        return HttpResponse::BadRequest().finish();
    }

    if Uuid::parse_str(&user_id).is_err() {
        return HttpResponse::BadRequest().finish();
    }

    let user_id = Uuid::parse_str(&user_id).unwrap();
    let rows = sqlx::query_as::<_, Activity>(
        "SELECT a.* FROM activities a WHERE a.user_id = $1 ORDER BY a.date DESC",
    )
    .bind(user_id)
    .fetch_all(db.get_ref())
    .await;

    if rows.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    let activities = rows.unwrap();
    let aggregation = aggretate_activities(&activities);

    HttpResponse::Ok().json(ActivitiesResponse {
        activities,
        aggregation: Some(aggregation),
    })
}

#[utoipa::path(
    get,
    path = "/trackpoints/:activity_id",
    params(
        ("activity_id" = String, description = "Activity ID", example = "123e4567-e89b-12d3-a456-426614174000")
    ),
    responses(
        (status = 200, description = "List all track points", body = ActivitiesResponse, content_type = "application/json")
    )
)]
#[get("/trackpoints/{activity_id}")]
pub async fn get_trackpoints(path: web::Path<String>, db: web::Data<PgPool>) -> impl Responder {
    let activity_id = path.into_inner();

    if activity_id.is_empty() {
        return HttpResponse::BadRequest().finish();
    }

    if Uuid::parse_str(&activity_id).is_err() {
        return HttpResponse::BadRequest().finish();
    }

    let activity_id = Uuid::parse_str(&activity_id).unwrap();

    let rows = sqlx::query_as::<_, TrackPoint>(
        "SELECT tp.id, tp.activity_id, tp.id AS trackpoint_id, tp.lat as latitude, tp.lon as longitude, tp.elevation, tp.time FROM trackpoints tp WHERE tp.activity_id = $1 ORDER BY tp.time DESC"
    )
    .bind(activity_id)
    .fetch_all(db.get_ref())
    .await;

    if rows.is_err() {
        println!("Error fetching track points: {:?}", rows.err());
        return HttpResponse::InternalServerError().finish();
    }

    let track_points = rows.unwrap();

    HttpResponse::Ok().json(track_points)
}

#[utoipa::path(
    post,
    path = "/activities",
    request_body(content = Vec<NewActivity>, description = "New activities to insert", content_type = "application/json"),
    responses(
        (status = 201, description = "Activities created")
    )
)]
#[post("/activities")]
pub async fn post_activities(
    db: web::Data<PgPool>,
    items: web::Json<Vec<NewActivity>>,
) -> impl Responder {
    for item in items.into_inner() {
        let _ = sqlx::query("INSERT INTO activities (id, name, time) VALUES ($1, $2, $3)")
            .bind(Uuid::new_v4())
            .bind(item.name)
            .bind(item.time)
            .execute(db.get_ref())
            .await;
    }

    HttpResponse::Created().finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activities::models::{NewActivity, TrackPoint};
    use actix_web::{test, App};
    use chrono::Utc;
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

    // #[actix_web::test]
    #[allow(dead_code)]
    async fn test_post_and_get_activities() {
        let db = setup_db().await;
        let user_id = Uuid::new_v4();

        let activity = NewActivity {
            name: "Test Activity".to_string(),
            time: Utc::now().naive_utc(),
        };

        let app = test::init_service(
            App::new()
                .app_data(actix_web::web::Data::new(db.clone()))
                .service(post_activities)
                .service(get_activities),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/activities")
            .set_json(vec![activity.clone()])
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 201);

        let req = test::TestRequest::get()
            .uri(&format!("/activities/{}", user_id))
            .to_request();

        let resp: ActivitiesResponse = test::call_and_read_body_json(&app, req).await;
        assert!(resp.activities.iter().any(|a| a.name == "Test Activity"));
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
            .uri("/activities/invalid-uuid")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }
}
