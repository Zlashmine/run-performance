use activities::models::{Activity, NewActivity, TrackPoint};
use utoipa::OpenApi;

mod activities;
mod aggregate;
mod api;
mod db;
mod users;

#[derive(OpenApi)]
#[allow(dead_code)]
#[openapi(
    paths(
        activities::get_activities,
        activities::get_trackpoints,
        activities::post_activities,
        users::get_user,
    ),
    components(schemas(Activity, NewActivity, TrackPoint)),
    tags(
        (name = "Activities", description = "Activity management endpoints")
    )
)]
struct ApiDoc;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let db_pool = db::init_db().await;

    return api::run_api(db_pool).await;
}
