use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::AppError,
    xp::{models::AwardXpInput, service as xp_service},
};

use super::{
    models::{
        category_display, parse_duration_to_secs, PersonalRecordSummary, PersonalRecordsResponse,
        PrCategorySummary, CATEGORIES,
    },
    repository,
};

pub async fn get_user_prs(
    db: &PgPool,
    user_id: Uuid,
) -> Result<PersonalRecordsResponse, AppError> {
    let records = repository::get_all_prs(db, user_id).await?;

    let summaries = CATEGORIES
        .iter()
        .map(|(slug, _, _)| {
            let pr = records.iter().find(|r| r.category == *slug);
            PersonalRecordSummary {
                category: slug.to_string(),
                category_display: category_display(slug).to_string(),
                distance_m: pr.map(|r| r.distance_m),
                duration_seconds: pr.map(|r| r.duration_seconds),
                pace_seconds_per_km: pr.map(|r| r.pace_seconds_per_km),
                achieved_at: pr.map(|r| r.achieved_at),
                activity_id: pr.and_then(|r| r.activity_id),
            }
        })
        .collect();

    Ok(PersonalRecordsResponse { records: summaries })
}

/// Checks an activity against all PR categories.  Persists any new/improved
/// records, awards 150 XP per PR, and returns compact summaries.
pub async fn check_activity_for_prs(
    db: &PgPool,
    user_id: Uuid,
    activity_id: Uuid,
    distance_m: f64,
    duration_str: &str,
    achieved_at: DateTime<Utc>,
) -> Result<Vec<PrCategorySummary>, AppError> {
    if distance_m <= 0.0 {
        return Ok(vec![]);
    }

    let duration_seconds = parse_duration_to_secs(duration_str);
    if duration_seconds <= 0 {
        return Ok(vec![]);
    }

    let pace_seconds_per_km = duration_seconds as f64 / (distance_m / 1000.0);
    let mut new_prs = Vec::new();

    for (slug, min_m, max_m) in CATEGORIES {
        let qualifies = if *slug == "longest_run" {
            // Longest run: no range, always eligible — the upsert handles the comparison.
            true
        } else {
            let min = min_m.unwrap();
            let max = max_m.unwrap();
            distance_m >= min && distance_m <= max
        };

        if !qualifies {
            continue;
        }

        // Check if this is a new PR (to determine is_first_pr after upsert).
        let existing = repository::get_pr(db, user_id, slug).await?;
        let is_first = existing.is_none();

        let upserted = repository::upsert_pr(
            db,
            user_id,
            slug,
            activity_id,
            distance_m,
            duration_seconds,
            pace_seconds_per_km,
            achieved_at,
        )
        .await?;

        if let Some(_record) = upserted {
            // Award 150 XP per PR set or improved.
            let xp_input = AwardXpInput {
                user_id,
                source_type: "pr".to_string(),
                source_id: Some(activity_id),
                xp_amount: 150,
                description: format!("Personal Record: {}", category_display(slug)),
            };
            if let Err(e) = xp_service::award_xp(db, xp_input).await {
                tracing::warn!("Failed to award XP for PR {}: {e}", slug);
            }

            new_prs.push(PrCategorySummary {
                category: slug.to_string(),
                category_display: category_display(slug).to_string(),
                pace_seconds_per_km,
                is_first_pr: is_first,
            });
        }
    }

    Ok(new_prs)
}
