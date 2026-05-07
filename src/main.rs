mod achievements;
mod activities;
mod aggregate;
mod api;
mod challenges;
mod db;
mod error;
mod missions;
mod monthly_missions;
mod personal_records;
mod strava;
mod sync;
mod users;
mod weekly_missions;
mod xp;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Connecting to database…");
    let db_pool = db::init_db().await;
    tracing::info!("Database connection established.");

    api::run_api(db_pool).await
}
