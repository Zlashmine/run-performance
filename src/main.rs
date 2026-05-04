mod activities;
mod aggregate;
mod api;
mod challenges;
mod db;
mod error;
mod users;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let db_pool = db::init_db().await;

    api::run_api(db_pool).await
}
