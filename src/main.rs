use activities::models::{Activity, NewActivity, TrackPoint};
use std::env;
use tracing::info;
use utoipa::OpenApi;

mod activities;
mod aggregate;
mod api;
mod cli;
mod db;
mod file_utils;
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

    if is_cli_mode() {
        let args: Vec<String> = env::args().collect();
        if args.len() < 3 {
            eprintln!("Please provide a folder name: cargo run -- --cli <folder>");
            std::process::exit(1);
        }

        let folder = &args[2];
        let start = std::time::Instant::now();

        cli::run_cli(folder, db_pool.clone()).await;

        let duration = start.elapsed();
        info!("Execution time: {}ms", duration.as_millis());

        return Ok(());
    }

    return api::run_api(db_pool).await;
}

fn is_cli_mode() -> bool {
    let args: Vec<String> = env::args().collect();
    args.len() > 1 && args[1] == "--cli"
}
