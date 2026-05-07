use std::collections::HashSet;

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

use super::models::{AchievementDefinition, AchievementWithStatus};

// ── Joined view row (manual mapping from dynamic query) ───────────────────────

struct AchievementRow {
    slug: String,
    name: String,
    description: String,
    icon: String,
    xp_reward: i32,
    rarity: String,
    category: String,
    is_secret: bool,
    unlocked_at: Option<chrono::DateTime<chrono::Utc>>,
    activity_id: Option<Uuid>,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for AchievementRow {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(Self {
            slug: row.try_get("slug")?,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            icon: row.try_get("icon")?,
            xp_reward: row.try_get("xp_reward")?,
            rarity: row.try_get("rarity")?,
            category: row.try_get("category")?,
            is_secret: row.try_get("is_secret")?,
            unlocked_at: row.try_get("unlocked_at")?,
            activity_id: row.try_get("activity_id")?,
        })
    }
}

/// Returns all achievement definitions with unlock status for the given user.
pub async fn get_achievements_for_user(
    db: &PgPool,
    user_id: Uuid,
) -> Result<Vec<AchievementWithStatus>, AppError> {
    let rows = sqlx::query_as::<_, AchievementRow>(
        "SELECT \
            ad.slug, ad.name, ad.description, ad.icon, ad.xp_reward, \
            ad.rarity, ad.category, ad.is_secret, \
            ua.unlocked_at, ua.activity_id \
         FROM achievement_definitions ad \
         LEFT JOIN user_achievements ua \
             ON ua.achievement_id = ad.id AND ua.user_id = $1 \
         ORDER BY ua.unlocked_at DESC NULLS LAST, ad.sort_order ASC",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(AppError::from)?;

    let achievements = rows
        .into_iter()
        .map(|row| {
            let unlocked = row.unlocked_at.is_some();
            let (display_name, display_description, display_icon) =
                if row.is_secret && !unlocked {
                    ("???".to_string(), "Keep running to discover".to_string(), "Lock".to_string())
                } else {
                    (row.name, row.description, row.icon)
                };
            AchievementWithStatus {
                slug: row.slug,
                name: display_name,
                description: display_description,
                icon: display_icon,
                xp_reward: row.xp_reward,
                rarity: row.rarity,
                category: row.category,
                is_secret: row.is_secret,
                unlocked,
                unlocked_at: row.unlocked_at,
                activity_id: row.activity_id,
            }
        })
        .collect();

    Ok(achievements)
}

/// Returns slugs already unlocked by a user.
pub async fn get_unlocked_slugs(
    db: &PgPool,
    user_id: Uuid,
) -> Result<HashSet<String>, AppError> {
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT ad.slug \
         FROM user_achievements ua \
         JOIN achievement_definitions ad ON ad.id = ua.achievement_id \
         WHERE ua.user_id = $1",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(AppError::from)?;

    Ok(rows.into_iter().collect())
}

/// Unlock the achievements for the given slugs (ON CONFLICT DO NOTHING).
/// Returns the definitions of the newly inserted rows only.
pub async fn unlock_achievements(
    db: &PgPool,
    user_id: Uuid,
    activity_id: Uuid,
    slugs: &[&str],
) -> Result<Vec<AchievementDefinition>, AppError> {
    if slugs.is_empty() {
        return Ok(vec![]);
    }

    let defs = sqlx::query_as::<_, AchievementDefinition>(
        "SELECT * FROM achievement_definitions WHERE slug = ANY($1)",
    )
    .bind(slugs)
    .fetch_all(db)
    .await
    .map_err(AppError::from)?;

    for def in &defs {
        sqlx::query(
            "INSERT INTO user_achievements (user_id, achievement_id, activity_id) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (user_id, achievement_id) DO NOTHING",
        )
        .bind(user_id)
        .bind(def.id)
        .bind(activity_id)
        .execute(db)
        .await
        .map_err(AppError::from)?;
    }

    Ok(defs)
}

// ── Aggregate helpers needed for CheckContext ─────────────────────────────────

pub async fn count_total_runs(db: &PgPool, user_id: Uuid) -> Result<i64, AppError> {
    let count: Option<i64> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activities WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .map_err(AppError::from)?;
    Ok(count.unwrap_or(0))
}

pub async fn sum_total_distance(db: &PgPool, user_id: Uuid) -> Result<f64, AppError> {
    let total: Option<f64> = sqlx::query_scalar(
        // distance column is in km; multiply by 1000 to return metres (callers compare against metre-based thresholds)
        "SELECT COALESCE(SUM(distance::double precision) * 1000.0, 0.0) FROM activities WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .map_err(AppError::from)?;
    Ok(total.unwrap_or(0.0))
}

pub async fn get_recent_paces(
    db: &PgPool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<f64>, AppError> {
    let rows: Vec<Option<f64>> = sqlx::query_scalar(
        "SELECT average_pace::double precision FROM activities \
         WHERE user_id = $1 ORDER BY date DESC LIMIT $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(db)
    .await
    .map_err(AppError::from)?;

    Ok(rows.into_iter().flatten().collect())
}

/// Returns the current consecutive-day run streak for the user.
pub async fn get_current_streak(db: &PgPool, user_id: Uuid) -> Result<i32, AppError> {
    let dates: Vec<Option<chrono::NaiveDate>> = sqlx::query_scalar(
        "SELECT DISTINCT DATE(date) FROM activities WHERE user_id = $1 ORDER BY 1 DESC",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(AppError::from)?;

    let dates: Vec<chrono::NaiveDate> = dates.into_iter().flatten().collect();

    let mut streak = 0i32;
    let today = chrono::Utc::now().date_naive();
    let mut expected = today;

    for date in &dates {
        if *date == expected || *date == expected - chrono::Duration::days(1) {
            if *date != expected {
                expected = *date;
            }
            streak += 1;
            expected -= chrono::Duration::days(1);
        } else {
            break;
        }
    }

    Ok(streak)
}

/// Returns true if there was a 30+ day gap before the most recent run.
pub async fn had_long_gap_before_latest(
    db: &PgPool,
    user_id: Uuid,
) -> Result<bool, AppError> {
    let dates: Vec<Option<chrono::NaiveDate>> = sqlx::query_scalar(
        "SELECT DISTINCT DATE(date) FROM activities WHERE user_id = $1 ORDER BY 1 DESC LIMIT 2",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(AppError::from)?;

    let dates: Vec<chrono::NaiveDate> = dates.into_iter().flatten().collect();

    if dates.len() < 2 {
        return Ok(false);
    }

    Ok((dates[0] - dates[1]).num_days() >= 30)
}

pub async fn get_months_with_runs(
    db: &PgPool,
    user_id: Uuid,
) -> Result<HashSet<(i32, u32)>, AppError> {
    struct YearMonth { year: Option<f64>, month: Option<f64> }

    impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for YearMonth {
        fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
            use sqlx::Row;
            Ok(Self {
                year: row.try_get("year")?,
                month: row.try_get("month")?,
            })
        }
    }

    let rows = sqlx::query_as::<_, YearMonth>(
        "SELECT EXTRACT(YEAR FROM date)::FLOAT8 AS year, EXTRACT(MONTH FROM date)::FLOAT8 AS month \
         FROM activities WHERE user_id = $1 \
         GROUP BY year, month",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(AppError::from)?;

    Ok(rows
        .into_iter()
        .filter_map(|r| match (r.year, r.month) {
            (Some(y), Some(m)) => Some((y as i32, m as u32)),
            _ => None,
        })
        .collect())
}

pub async fn count_monday_runs(db: &PgPool, user_id: Uuid) -> Result<i64, AppError> {
    let count: Option<i64> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activities \
         WHERE user_id = $1 AND EXTRACT(DOW FROM date) = 1",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .map_err(AppError::from)?;
    Ok(count.unwrap_or(0))
}

/// Returns the PR count if the personal_records table exists, else 0.
pub async fn count_personal_records(db: &PgPool, user_id: Uuid) -> Result<i64, AppError> {
    let exists: Option<i64> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_name = 'personal_records'",
    )
    .fetch_one(db)
    .await
    .map_err(AppError::from)?;

    if exists.unwrap_or(0) == 0 {
        return Ok(0);
    }

    let count: Option<i64> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM personal_records WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(db)
    .await
    .map_err(AppError::from)?;
    Ok(count.unwrap_or(0))
}
