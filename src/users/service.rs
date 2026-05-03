/// Business logic layer for the users domain.
///
/// Thin orchestration between handlers and repository.
/// All validation belongs in handlers; all SQL belongs in repository.
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

use super::{
    models::{CreateUser, User},
    repository,
};

pub async fn get_user(db: &PgPool, user_id: Uuid) -> Result<User, AppError> {
    repository::find_by_id(db, user_id)
        .await?
        .ok_or(AppError::NotFound)
}

pub async fn upsert_user(db: &PgPool, payload: &CreateUser) -> Result<User, AppError> {
    repository::upsert(db, payload).await
}
