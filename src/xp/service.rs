use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

use super::{
    models::{level_bounds, level_from_xp, AwardXpInput, UserXpResponse},
    repository,
};

pub async fn get_user_xp_summary(db: &PgPool, user_id: Uuid) -> Result<UserXpResponse, AppError> {
    let mut xp_row = repository::get_or_create_user_xp(db, user_id).await?;

    // Lazy retroactive seed on first visit
    if !xp_row.initialized {
        repository::seed_retroactive_xp(db, user_id).await?;
        xp_row = repository::get_or_create_user_xp(db, user_id).await?;
    }

    let recent_events = repository::get_recent_events(db, user_id, 10).await?;

    let (level, level_name) = level_from_xp(xp_row.xp_total);
    let (xp_for_current_level, xp_for_next_level, next_level_name) =
        level_bounds(xp_row.xp_total);

    let xp_in_current_level = xp_row.xp_total - xp_for_current_level;
    let range = (xp_for_next_level - xp_for_current_level).max(1) as f64;
    let progress_percent = ((xp_in_current_level as f64 / range) * 100.0).clamp(0.0, 100.0);

    Ok(UserXpResponse {
        xp_total: xp_row.xp_total,
        level,
        level_name: level_name.to_string(),
        xp_for_current_level,
        xp_for_next_level,
        xp_in_current_level,
        progress_percent,
        next_level_name: next_level_name.to_string(),
        recent_events,
    })
}

/// Award XP — public API used by activities, achievements, missions, PRs.
pub async fn award_xp(db: &PgPool, input: AwardXpInput) -> Result<(), AppError> {
    repository::award_xp(db, input).await?;
    Ok(())
}
