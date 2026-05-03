/// SQL layer for the users domain.
///
/// Every function takes `&PgPool` and returns `Result<_, AppError>`.
/// Never used directly by handlers — go through `service.rs`.
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

use super::models::{CreateUser, User};

pub async fn find_by_id(db: &PgPool, user_id: Uuid) -> Result<Option<User>, AppError> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(db)
        .await
        .map_err(AppError::from)
}

/// Upsert a user by email.
///
/// Uses `ON CONFLICT (email) DO UPDATE` so a single query handles both
/// create-new and return-existing cases. The google_id is updated in case
/// it changes between logins (e.g. token refresh).
pub async fn upsert(db: &PgPool, payload: &CreateUser) -> Result<User, AppError> {
    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (id, google_id, email, created_at)
        VALUES (gen_random_uuid(), $1, $2, now())
        ON CONFLICT (email) DO UPDATE
            SET google_id = EXCLUDED.google_id
        RETURNING *
        "#,
    )
    .bind(&payload.google_id)
    .bind(&payload.email)
    .fetch_one(db)
    .await
    .map_err(AppError::from)
}
