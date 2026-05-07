use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::AppError,
    xp::{models::AwardXpInput, service as xp_service},
};

use super::{
    definitions::{evaluate_all, CheckContext},
    models::{AchievementWithStatus, UnlockedAchievementSummary},
    repository,
};

pub async fn get_user_achievements(
    db: &PgPool,
    user_id: Uuid,
) -> Result<Vec<AchievementWithStatus>, AppError> {
    repository::get_achievements_for_user(db, user_id).await
}

/// Called after every activity upload.
///
/// Builds context from the DB, evaluates rules, persists new unlocks, awards XP,
/// and returns compact summaries of any newly-unlocked achievements.
pub async fn check_and_unlock_achievements(
    db: &PgPool,
    user_id: Uuid,
    activity_id: Uuid,
    distance_m: f64,
    pace_min_per_km: f64,
    activity_start: DateTime<Utc>,
) -> Result<Vec<UnlockedAchievementSummary>, AppError> {
    // Gather context.
    let (
        total_runs,
        total_distance_m,
        current_streak,
        recent_paces,
        already_unlocked,
        months_with_runs,
        pr_count,
        monday_run_count,
        had_long_gap,
    ) = tokio::join!(
        repository::count_total_runs(db, user_id),
        repository::sum_total_distance(db, user_id),
        repository::get_current_streak(db, user_id),
        repository::get_recent_paces(db, user_id, 10),
        repository::get_unlocked_slugs(db, user_id),
        repository::get_months_with_runs(db, user_id),
        repository::count_personal_records(db, user_id),
        repository::count_monday_runs(db, user_id),
        repository::had_long_gap_before_latest(db, user_id),
    );

    let ctx = CheckContext {
        user_id,
        activity_id,
        activity_start,
        activity_distance_m: distance_m,
        activity_pace_min_per_km: pace_min_per_km,
        total_runs: total_runs.unwrap_or(0),
        total_distance_m: total_distance_m.unwrap_or(0.0),
        current_streak: current_streak.unwrap_or(0),
        recent_paces: recent_paces.unwrap_or_default(),
        already_unlocked: already_unlocked.unwrap_or_default(),
        months_with_runs: months_with_runs.unwrap_or_default(),
        pr_count: pr_count.unwrap_or(0),
        monday_run_count: monday_run_count.unwrap_or(0),
        had_long_gap: had_long_gap.unwrap_or(false),
    };

    let newly_earned_slugs = evaluate_all(&ctx);
    if newly_earned_slugs.is_empty() {
        return Ok(vec![]);
    }

    let unlocked_defs =
        repository::unlock_achievements(db, user_id, activity_id, &newly_earned_slugs).await?;

    // Award XP for each newly unlocked achievement (non-fatal on failure).
    for def in &unlocked_defs {
        let input = AwardXpInput {
            user_id,
            source_type: "achievement".to_string(),
            source_id: Some(def.id),
            xp_amount: def.xp_reward,
            description: format!("Achievement: {}", def.name),
        };
        if let Err(e) = xp_service::award_xp(db, input).await {
            tracing::warn!("Failed to award XP for achievement {}: {e}", def.slug);
        }
    }

    let summaries = unlocked_defs
        .into_iter()
        .map(|d| UnlockedAchievementSummary {
            slug: d.slug,
            name: d.name,
            icon: d.icon,
            rarity: d.rarity,
            xp_reward: d.xp_reward,
        })
        .collect();

    Ok(summaries)
}
