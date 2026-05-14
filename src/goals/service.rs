/// Business logic for user-defined goals.
use chrono::{Datelike, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    activities,
    error::AppError,
    xp::{models::AwardXpInput, service as xp_service},
};

use super::{
    models::{
        CompletedGoalSummary, CreateGoalRequest, ParsedRequirements, UserGoal, UserGoalResponse,
    },
    repository,
    requirement_type::{GoalFilterType, GoalMetricType},
};

// ─── Period helpers ───────────────────────────────────────────────────────────

fn current_period_key(timeframe: &str) -> String {
    let now = Utc::now();
    match timeframe {
        "monthly" => format!("{}-{:02}", now.year(), now.month()),
        "yearly" => format!("{}", now.year()),
        _ => String::new(), // "forever"
    }
}

fn period_window(timeframe: &str, period_key: &str) -> (chrono::DateTime<Utc>, Option<chrono::DateTime<Utc>>) {
    use chrono::{NaiveDate, TimeZone};

    match timeframe {
        "monthly" => {
            // period_key = "YYYY-MM"
            let parts: Vec<&str> = period_key.splitn(2, '-').collect();
            let (year, month) = if parts.len() == 2 {
                (
                    parts[0].parse::<i32>().unwrap_or(2025),
                    parts[1].parse::<u32>().unwrap_or(1),
                )
            } else {
                let now = Utc::now();
                (now.year(), now.month())
            };
            let start = NaiveDate::from_ymd_opt(year, month, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap();
            // End = first day of next month
            let (next_year, next_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
            let end = NaiveDate::from_ymd_opt(next_year, next_month, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap();
            (
                Utc.from_utc_datetime(&start),
                Some(Utc.from_utc_datetime(&end)),
            )
        }
        "yearly" => {
            let year = period_key.parse::<i32>().unwrap_or_else(|_| Utc::now().year());
            let start = NaiveDate::from_ymd_opt(year, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap();
            let end = NaiveDate::from_ymd_opt(year + 1, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap();
            (
                Utc.from_utc_datetime(&start),
                Some(Utc.from_utc_datetime(&end)),
            )
        }
        _ => {
            // "forever" — all activities
            (chrono::DateTime::UNIX_EPOCH, None)
        }
    }
}

// ─── Filter application ───────────────────────────────────────────────────────

fn activity_passes_filters(
    activity: &activities::models::Activity,
    filters: &[(GoalFilterType, Option<f64>, serde_json::Value)],
) -> bool {
    for (filter_type, value, params) in filters {
        let passes = match filter_type {
            GoalFilterType::ActivityTypeIs => {
                let expected = params
                    .get("activity_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                activity.activity_type.eq_ignore_ascii_case(expected)
            }
            GoalFilterType::MinDistance => {
                value.map_or(true, |v| activity.distance as f64 >= v)
            }
            GoalFilterType::MaxDistance => {
                value.map_or(true, |v| activity.distance as f64 <= v)
            }
            GoalFilterType::MinDuration => {
                // activity.duration is stored as "HH:MM:SS"
                let secs = parse_duration_secs(&activity.duration);
                value.map_or(true, |v| secs as f64 >= v * 60.0)
            }
            GoalFilterType::MinPace => {
                // min_pace: only activities with pace <= value (faster than threshold)
                value.map_or(true, |v| activity.average_pace as f64 <= v)
            }
            GoalFilterType::MaxPace => {
                // max_pace: only activities with pace >= value (slower than threshold)
                value.map_or(true, |v| activity.average_pace as f64 >= v)
            }
            GoalFilterType::MinElevation => {
                value.map_or(true, |v| activity.climb as f64 >= v)
            }
        };
        if !passes {
            return false;
        }
    }
    true
}

fn parse_duration_secs(duration: &str) -> u64 {
    // Format: "HH:MM:SS" or "MM:SS"
    let parts: Vec<u64> = duration
        .split(':')
        .filter_map(|p| p.parse().ok())
        .collect();
    match parts.len() {
        3 => parts[0] * 3600 + parts[1] * 60 + parts[2],
        2 => parts[0] * 60 + parts[1],
        1 => parts[0],
        _ => 0,
    }
}

// ─── Metric aggregation ───────────────────────────────────────────────────────

fn aggregate_metric(
    metric: GoalMetricType,
    activities: &[&activities::models::Activity],
) -> f64 {
    if activities.is_empty() {
        return 0.0;
    }
    match metric {
        GoalMetricType::TotalDistance => activities.iter().map(|a| a.distance as f64).sum(),
        GoalMetricType::TotalDuration => activities
            .iter()
            .map(|a| parse_duration_secs(&a.duration) as f64 / 60.0) // minutes
            .sum(),
        GoalMetricType::TotalActivities => activities.len() as f64,
        GoalMetricType::TotalElevation => activities.iter().map(|a| a.climb as f64).sum(),
        GoalMetricType::TotalCalories => activities.iter().map(|a| a.calories as f64).sum(),
        GoalMetricType::LongestRun => activities
            .iter()
            .map(|a| a.distance as f64)
            .fold(0.0_f64, f64::max),
        GoalMetricType::FastestPace => activities
            .iter()
            .map(|a| a.average_pace as f64)
            .filter(|&p| p > 0.0)
            .fold(f64::MAX, f64::min),
        GoalMetricType::AveragePace => {
            let paces: Vec<f64> = activities
                .iter()
                .map(|a| a.average_pace as f64)
                .filter(|&p| p > 0.0)
                .collect();
            if paces.is_empty() {
                0.0
            } else {
                paces.iter().sum::<f64>() / paces.len() as f64
            }
        }
    }
}

// ─── Requirement parsing ──────────────────────────────────────────────────────

fn parse_requirements(
    reqs: &[super::models::CreateGoalRequirementRequest],
) -> Result<ParsedRequirements, AppError> {
    let metric_reqs: Vec<_> = reqs.iter().filter(|r| r.category == "metric").collect();
    if metric_reqs.len() != 1 {
        return Err(AppError::BadRequest(
            "Exactly one metric requirement is required".to_string(),
        ));
    }
    let metric = metric_reqs[0]
        .requirement_type
        .parse::<GoalMetricType>()
        .map_err(|e| AppError::BadRequest(e))?;

    let mut filters = vec![];
    for f in reqs.iter().filter(|r| r.category == "filter") {
        let ft = f
            .requirement_type
            .parse::<GoalFilterType>()
            .map_err(|e| AppError::BadRequest(e))?;
        filters.push((ft, f.value, f.params.clone()));
    }

    Ok(ParsedRequirements { metric, filters })
}

fn extract_requirements_from_db(
    reqs: &[super::models::GoalRequirement],
) -> Option<(GoalMetricType, Vec<(GoalFilterType, Option<f64>, serde_json::Value)>)> {
    let metric_req = reqs.iter().find(|r| r.category == "metric")?;
    let metric = metric_req.requirement_type.parse::<GoalMetricType>().ok()?;

    let filters = reqs
        .iter()
        .filter(|r| r.category == "filter")
        .filter_map(|r| {
            let ft = r.requirement_type.parse::<GoalFilterType>().ok()?;
            Some((ft, r.value, r.params.clone()))
        })
        .collect();

    Some((metric, filters))
}

// ─── Shared progress computation ─────────────────────────────────────────────

/// Compute the current metric value for a goal against existing activities,
/// persist it to the DB, and award XP if the goal is immediately met.
/// Returns `(new_value, completed_at)`.
async fn compute_and_persist_goal(
    db: &PgPool,
    user_id: Uuid,
    goal: &UserGoal,
    reqs: &[super::models::GoalRequirement],
    now: chrono::DateTime<Utc>,
) -> (f64, Option<chrono::DateTime<Utc>>) {
    let (metric, filters) = match extract_requirements_from_db(reqs) {
        Some(r) => r,
        None => {
            tracing::warn!("Goal {} has no valid metric requirement; skipping", goal.id);
            return (goal.current_value, goal.completed_at);
        }
    };

    let current_key = current_period_key(&goal.timeframe);
    let (from, to) = period_window(&goal.timeframe, &current_key);

    let all_activities =
        activities::repository::find_activities_by_user_from(db, user_id, from, to)
            .await
            .unwrap_or_default();

    let filtered: Vec<&activities::models::Activity> = all_activities
        .iter()
        .filter(|a| activity_passes_filters(a, &filters))
        .collect();

    let new_value = aggregate_metric(metric, &filtered);
    let new_completed_at = if metric.is_met(new_value, goal.target_value) {
        Some(now)
    } else {
        None
    };

    repository::update_goal_progress(db, goal.id, new_value, &current_key, new_completed_at)
        .await
        .unwrap_or_else(|e| tracing::warn!("Failed to update goal {} progress: {e}", goal.id));

    // Award XP if this computation marks the goal as newly completed.
    if new_completed_at.is_some() && goal.completed_at.is_none() {
        let xp_input = AwardXpInput {
            user_id,
            source_type: "goal".to_string(),
            source_id: Some(goal.id),
            xp_amount: goal.xp_reward,
            description: format!("Goal completed: {}", goal.name),
        };
        if let Err(e) = xp_service::award_xp(db, xp_input).await {
            tracing::warn!("Failed to award XP for goal {}: {e}", goal.id);
        }
    }

    (new_value, new_completed_at)
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Return all goals for a user, applying lazy period-rollover so the UI
/// always sees a fresh slate after a period boundary even without an upload.
pub async fn list_goals(
    db: &PgPool,
    user_id: Uuid,
) -> Result<Vec<UserGoalResponse>, AppError> {
    let pairs = repository::find_goals_for_user(db, user_id).await?;
    let current_now = Utc::now();

    Ok(pairs
        .into_iter()
        .map(|(goal, reqs)| {
            let (display_value, display_completed) =
                apply_lazy_rollover(&goal, &current_now);

            UserGoalResponse {
                id: goal.id,
                user_id: goal.user_id,
                name: goal.name,
                description: goal.description,
                timeframe: goal.timeframe,
                period_key: goal.period_key,
                current_value: display_value,
                target_value: goal.target_value,
                completed_at: display_completed,
                xp_reward: goal.xp_reward,
                requirements: reqs,
                created_at: goal.created_at,
                updated_at: goal.updated_at,
            }
        })
        .collect())
}

/// Returns `(current_value, completed_at)` adjusted for period rollover.
/// Does NOT write to DB — purely for display purposes.
fn apply_lazy_rollover(
    goal: &UserGoal,
    now: &chrono::DateTime<Utc>,
) -> (f64, Option<chrono::DateTime<Utc>>) {
    let current_key = match goal.timeframe.as_str() {
        "monthly" => format!("{}-{:02}", now.year(), now.month()),
        "yearly" => format!("{}", now.year()),
        _ => return (goal.current_value, goal.completed_at), // forever — no rollover
    };

    if goal.period_key != current_key {
        // Period has rolled over; show zeroed state without a DB write
        (0.0, None)
    } else {
        (goal.current_value, goal.completed_at)
    }
}

/// Create a goal for a user, enforcing the 3-slot limit within a transaction.
pub async fn create_goal(
    db: &PgPool,
    user_id: Uuid,
    req: CreateGoalRequest,
) -> Result<UserGoalResponse, AppError> {
    // Validate input at the boundary.
    if req.name.is_empty() || req.name.len() > 100 {
        return Err(AppError::BadRequest(
            "Goal name must be between 1 and 100 characters".to_string(),
        ));
    }
    if req.target_value <= 0.0 {
        return Err(AppError::BadRequest(
            "target_value must be greater than 0".to_string(),
        ));
    }
    if !["monthly", "yearly", "forever"].contains(&req.timeframe.as_str()) {
        return Err(AppError::BadRequest(
            "timeframe must be 'monthly', 'yearly', or 'forever'".to_string(),
        ));
    }
    let parsed = parse_requirements(&req.requirements)?;

    let period_key = current_period_key(&req.timeframe);
    let xp_reward = req.xp_reward.unwrap_or(150);

    // Use an advisory lock per user to prevent concurrent slot-limit races.
    // The lock is scoped to the transaction and released automatically on commit/rollback.
    let user_hash = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        user_id.hash(&mut h);
        h.finish() as i64
    };

    let mut tx = db.begin().await.map_err(AppError::from)?;

    // Advisory lock: serialises concurrent creates for this user.
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(user_hash)
        .execute(&mut *tx)
        .await
        .map_err(AppError::from)?;

    // Count slots inside the locked transaction.
    let slot_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM goals
         WHERE user_id = $1
           AND NOT (timeframe = 'forever' AND completed_at IS NOT NULL)",
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::from)?;

    if slot_count.0 >= 3 {
        return Err(AppError::BadRequest("goal_limit_reached".to_string()));
    }

    // Insert goal.
    let goal = sqlx::query_as::<_, UserGoal>(
        r#"
        INSERT INTO goals
            (user_id, name, description, timeframe, period_key, target_value, xp_reward)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(&req.name)
    .bind(req.description.as_deref())
    .bind(&req.timeframe)
    .bind(&period_key)
    .bind(req.target_value)
    .bind(xp_reward)
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::from)?;

    // Insert requirements.
    let req_rows: Vec<(String, String, Option<f64>, serde_json::Value)> = {
        let mut rows = vec![(
            "metric".to_string(),
            parsed.metric.as_str().to_string(),
            None,
            serde_json::Value::Object(serde_json::Map::new()),
        )];
        for (ft, value, params) in &parsed.filters {
            rows.push(("filter".to_string(), ft.as_str().to_string(), *value, params.clone()));
        }
        rows
    };

    for (category, req_type, value, params) in &req_rows {
        sqlx::query(
            "INSERT INTO goal_requirements (goal_id, category, requirement_type, value, params)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(goal.id)
        .bind(category)
        .bind(req_type)
        .bind(value)
        .bind(params)
        .execute(&mut *tx)
        .await
        .map_err(AppError::from)?;
    }

    tx.commit().await.map_err(AppError::from)?;

    // Re-fetch requirements for the response.
    let reqs = sqlx::query_as::<_, super::models::GoalRequirement>(
        "SELECT * FROM goal_requirements WHERE goal_id = $1 ORDER BY category DESC, created_at ASC",
    )
    .bind(goal.id)
    .fetch_all(db)
    .await
    .map_err(AppError::from)?;

    // Compute initial progress from activities that already exist in this period.
    let (initial_value, initial_completed_at) =
        compute_and_persist_goal(db, user_id, &goal, &reqs, Utc::now()).await;

    Ok(UserGoalResponse {
        id: goal.id,
        user_id: goal.user_id,
        name: goal.name,
        description: goal.description,
        timeframe: goal.timeframe,
        period_key: goal.period_key,
        current_value: initial_value,
        target_value: goal.target_value,
        completed_at: initial_completed_at,
        xp_reward: goal.xp_reward,
        requirements: reqs,
        created_at: goal.created_at,
        updated_at: goal.updated_at,
    })
}

/// Delete a goal, enforcing owner check.
pub async fn delete_goal(
    db: &PgPool,
    goal_id: Uuid,
    requesting_user_id: Uuid,
) -> Result<(), AppError> {
    let goal = repository::find_goal_by_id(db, goal_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if goal.user_id != requesting_user_id {
        return Err(AppError::Forbidden);
    }

    repository::delete_goal(db, goal_id).await
}

/// Called after activity upload. Recalculates progress for all active goals
/// of the user and awards XP for any newly completed goals.
///
/// Returns a list of goals that were completed during this upload.
/// Failure is non-fatal — callers wrap this in `unwrap_or_else`.
pub async fn update_progress_after_upload(
    db: &PgPool,
    user_id: Uuid,
) -> Result<Vec<CompletedGoalSummary>, AppError> {
    let pairs = repository::find_goals_for_user(db, user_id).await?;
    if pairs.is_empty() {
        return Ok(vec![]);
    }

    let now = Utc::now();
    let mut completed = vec![];

    for (mut goal, reqs) in pairs {
        // Skip permanently completed forever goals.
        if goal.timeframe == "forever" && goal.completed_at.is_some() {
            continue;
        }

        let current_key = current_period_key(&goal.timeframe);

        // Period rollover: reset DB-side values so the helper uses the fresh period.
        if goal.timeframe != "forever" && goal.period_key != current_key {
            goal.current_value = 0.0;
            goal.completed_at = None;
            goal.period_key = current_key;
        }

        // Skip already-completed goals in this period.
        if goal.completed_at.is_some() {
            continue;
        }

        // Delegate computation, persistence, and XP award to the shared helper.
        let (new_value, new_completed_at) =
            compute_and_persist_goal(db, user_id, &goal, &reqs, now).await;

        if new_completed_at.is_some() {
            completed.push(CompletedGoalSummary {
                goal_id: goal.id,
                name: goal.name.clone(),
                xp_earned: goal.xp_reward,
            });
        }

        let _ = new_value; // silences unused-variable lint
    }

    Ok(completed)
}
