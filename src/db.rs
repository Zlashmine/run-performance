use sqlx::postgres::PgPoolOptions;
use std::env;
use std::time::Duration;

pub async fn init_db() -> sqlx::PgPool {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL not set");

    let max_connections: u32 = env::var("DB_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);

    let min_connections: u32 = env::var("DB_MIN_CONNECTIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);

    let connect_timeout_secs: u64 = env::var("DB_CONNECT_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);

    PgPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(min_connections)
        .acquire_timeout(Duration::from_secs(connect_timeout_secs))
        .connect(&db_url)
        .await
        .expect("Failed to connect to DB")
}
