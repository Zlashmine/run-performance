use std::env;

use crate::activities::{
    self,
    models::{Activity, NewActivity},
};
use crate::users::{self};
use actix_cors::Cors;
use actix_web::http::header;
use actix_web::middleware::{NormalizePath, TrailingSlash};
use actix_web::{middleware::Logger, web, App, HttpServer};
use sqlx::PgPool;
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    paths(
        activities::get_activities,
        activities::get_activity_detail,
        activities::get_trackpoints,
        activities::post_activities,
        activities::upload_files,
        users::get_user,
        users::create_user,
    ),
    components(schemas(Activity, NewActivity)),
    tags(
        (name = "Activities", description = "Activity management endpoints")
    )
)]
struct ApiDoc;

pub async fn run_api(db_pool: PgPool) -> std::io::Result<()> {
    info!("Starting server....");

    // let governor_conf = GovernorConfigBuilder::default()
    //     .seconds_per_request(5)
    //     .burst_size(10)
    //     .finish()
    //     .unwrap();

    sqlx::migrate!()
        .run(&db_pool)
        .await
        .expect("Failed to run database migrations");

    info!("Database migrations applied successfully.");

    let port: u16 = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("PORT must be a valid u16");

    info!("Binding server to 0.0.0.0:{}", port);

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(NormalizePath::new(TrailingSlash::Trim))
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header(),
            )
            .wrap(
                actix_web::middleware::DefaultHeaders::new()
                    .add((
                        header::STRICT_TRANSPORT_SECURITY,
                        "max-age=63072000; includeSubDomains; preload",
                    ))
                    .add((header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
                    .add((header::X_FRAME_OPTIONS, "DENY"))
                    .add((header::X_XSS_PROTECTION, "1; mode=block")),
            )
            // .wrap(Governor::new(&governor_conf))
            .app_data(web::Data::new(db_pool.clone()))
            .service(activities::get_activities)
            .service(activities::get_activity_detail)
            .service(activities::get_trackpoints)
            .service(activities::upload_files)
            .service(users::get_user)
            .service(users::create_user)
            .service(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
    })
    .bind(("0.0.0.0", port))? // IMPORTANT: use 0.0.0.0 not 127.0.0.1
    .run()
    .await
}
