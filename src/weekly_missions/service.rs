use chrono::{Datelike, Duration, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::AppError,
    missions::common::{dow_name, CompletedMissionSummary},
    xp::{models::AwardXpInput, service as xp_service},
};

use super::{
    models::{WeeklyMission, WeeklyMissionsResponse},
    repository,
};

/// Returns the ISO Monday that starts the current week (UTC).
pub fn current_week_start() -> NaiveDate {
    let today = Utc::now().date_naive();
    let days_since_monday = today.weekday().num_days_from_monday();
    today - Duration::days(days_since_monday as i64)
}

/// Stats used to personalise mission generation.
struct UserWeeklyStats {
    avg_weekly_km: f64,
    avg_weekly_runs: f64,
    last_week_km: f64,
    most_skipped_dow: Option<u32>, // 0=Sun…6=Sat (PostgreSQL extract(dow))
    avg_pace_secs: f64,            // secs/km, across all activities
}

async fn fetch_weekly_stats(pool: &PgPool, user_id: Uuid, week_start: NaiveDate) -> UserWeeklyStats {
    // Average weekly km & run count (all historical weeks)
    // AVG on empty set returns one row with NULLs, so fetch_one is safe.
    let agg: (Option<f64>, Option<f64>) = sqlx::query_as::<_, (Option<f64>, Option<f64>)>(
        r#"
        SELECT
            AVG(weekly_km)    AS avg_weekly_km,
            AVG(weekly_count) AS avg_weekly_runs
        FROM (
            SELECT
                date_trunc('week', date) AS w,
                SUM(distance::FLOAT8) AS weekly_km,
                COUNT(*)              AS weekly_count
            FROM activities
            WHERE user_id = $1
            GROUP BY w
        ) weekly_agg
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or((None, None));

    let avg_weekly_km = agg.0.unwrap_or(5.0);
    let avg_weekly_runs = agg.1.unwrap_or(2.0);

    // Last week's total km
    let last_week_start = week_start - Duration::days(7);
    let last_week_end = week_start;
    let last_week_km: Option<f64> = sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(distance::FLOAT8), 0)
        FROM activities
        WHERE user_id = $1
          AND date >= $2
          AND date < $3
        "#,
    )
    .bind(user_id)
    .bind(last_week_start.and_hms_opt(0, 0, 0))
    .bind(last_week_end.and_hms_opt(0, 0, 0))
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    // Most skipped day of week (the weekday with fewest runs across all history)
    // Uses extract(dow): 0=Sunday, 1=Monday … 6=Saturday.
    let most_skipped_dow: Option<u32> = {
        let row: Option<(f64,)> = sqlx::query_as::<_, (f64,)>(
            r#"
            SELECT CAST(extract(dow FROM date) AS FLOAT) AS dow
            FROM activities
            WHERE user_id = $1
            GROUP BY CAST(extract(dow FROM date) AS FLOAT)
            ORDER BY COUNT(*) ASC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        row.map(|(v,)| v as u32)
    };

    // Average pace in secs/km (only for activities with positive pace)
    // average_pace is stored as M.SS (e.g. 5.12 means 5:12/km).
    // Convert: floor(m.ss) * 60 + round((m.ss - floor(m.ss)) * 100)
    // We approximate: average_pace * 60 works for rough generation.
    // We use a simpler approximation: average_pace (minutes decimal) * 60 = secs/km.
    let avg_pace: Option<f64> = sqlx::query_scalar(
        r#"
        SELECT AVG(CAST(average_pace AS FLOAT8))
        FROM activities
        WHERE user_id = $1 AND average_pace > 0
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .flatten();

    // Convert M.SS format to seconds: e.g. 5.12 -> 5*60 + 12 = 312
    let avg_pace_secs = avg_pace
        .map(|p| {
            let mins = p.floor();
            let secs = (p - mins) * 100.0;
            mins * 60.0 + secs
        })
        .unwrap_or(360.0); // default 6:00/km = 360 secs

    UserWeeklyStats {
        avg_weekly_km,
        avg_weekly_runs,
        last_week_km: last_week_km.unwrap_or(0.0),
        most_skipped_dow,
        avg_pace_secs,
    }
}

/// Generate 3 personalised missions for the given user/week.
fn generate_missions(
    user_id: Uuid,
    week_start: NaiveDate,
    stats: &UserWeeklyStats,
) -> Vec<WeeklyMission> {
    let now = Utc::now();
    let mut missions: Vec<WeeklyMission> = Vec::with_capacity(3);

    // Mission 1: always run_distance_km
    let target_km = (stats.avg_weekly_km * 1.1 * 10.0).round() / 10.0;
    let target_km = target_km.max(1.0);
    missions.push(WeeklyMission {
        id: Uuid::new_v4(),
        user_id,
        week_start,
        mission_type: "run_distance_km".to_string(),
        title: format!("Run {target_km}km this week"),
        description: format!("Accumulate {target_km}km of running this week"),
        target_value: target_km,
        current_value: 0.0,
        xp_reward: 100,
        completed_at: None,
        rerolled: false,
        created_at: now,
        updated_at: now,
    });

    // Mission 2: beat last week or run_sub_pace
    if stats.last_week_km > 0.5 {
        let last = (stats.last_week_km * 10.0).round() / 10.0;
        missions.push(WeeklyMission {
            id: Uuid::new_v4(),
            user_id,
            week_start,
            mission_type: "beat_last_week_km".to_string(),
            title: "Beat last week's distance".to_string(),
            description: format!("Run more than {last}km (your total from last week)"),
            target_value: last,
            current_value: 0.0,
            xp_reward: 100,
            completed_at: None,
            rerolled: false,
            created_at: now,
            updated_at: now,
        });
    } else {
        // Target pace 10 secs/km faster than average
        let target_pace_secs = (stats.avg_pace_secs - 10.0).max(180.0);
        let mins = (target_pace_secs / 60.0).floor() as u32;
        let secs = (target_pace_secs % 60.0).round() as u32;
        let pace_str = format!("{mins}:{secs:02}");
        missions.push(WeeklyMission {
            id: Uuid::new_v4(),
            user_id,
            week_start,
            mission_type: "run_sub_pace".to_string(),
            title: format!("Run sub {pace_str}/km for 5km+"),
            description: format!("Complete a 5km+ run under {pace_str}/km"),
            target_value: target_pace_secs,
            current_value: f64::MAX, // lower is better; starts "worst"
            xp_reward: 100,
            completed_at: None,
            rerolled: false,
            created_at: now,
            updated_at: now,
        });
    }

    // Mission 3: run_on_skipped_day or longest_single_run
    if let Some(dow) = stats.most_skipped_dow {
        let day_name = dow_name(dow);
        let current_dow = week_start.weekday().num_days_from_sunday();
        // Only do run_on_skipped_day if the day hasn't passed yet
        let target_dow = dow;
        missions.push(WeeklyMission {
            id: Uuid::new_v4(),
            user_id,
            week_start,
            mission_type: "run_on_skipped_day".to_string(),
            title: format!("Run on a {day_name}"),
            description: format!("You usually skip {day_name}. Change that this week!"),
            target_value: target_dow as f64,
            current_value: 0.0,
            xp_reward: 100,
            completed_at: None,
            rerolled: false,
            created_at: now,
            updated_at: now,
        });
        let _ = current_dow; // suppress warning
    } else {
        // longest_single_run: 5% more than average long run (rough: avg_weekly_km * 0.6)
        let target_long = ((stats.avg_weekly_km * 0.6 * 1.05 * 10.0).round() / 10.0).max(3.0);
        missions.push(WeeklyMission {
            id: Uuid::new_v4(),
            user_id,
            week_start,
            mission_type: "longest_single_run".to_string(),
            title: format!("Go long — {target_long}km or more"),
            description: format!("Complete a single run of at least {target_long}km"),
            target_value: target_long,
            current_value: 0.0,
            xp_reward: 100,
            completed_at: None,
            rerolled: false,
            created_at: now,
            updated_at: now,
        });
    }

    missions
}

/// Lazily generate missions for the current week, then return all three.
pub async fn get_or_generate_missions(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<WeeklyMissionsResponse, AppError> {
    let week_start = current_week_start();
    let existing = repository::get_missions_for_week(pool, user_id, week_start).await?;

    if existing.len() < 3 {
        // Generate missions for the types not yet present
        let existing_types: std::collections::HashSet<String> =
            existing.iter().map(|m| m.mission_type.clone()).collect();

        let stats = fetch_weekly_stats(pool, user_id, week_start).await;
        let generated = generate_missions(user_id, week_start, &stats);

        // Only insert types not already present
        let to_insert: Vec<WeeklyMission> = generated
            .into_iter()
            .filter(|m| !existing_types.contains(&m.mission_type))
            .collect();

        if !to_insert.is_empty() {
            repository::insert_missions(pool, &to_insert).await?;
        }
    }

    // Ensure progress is always up-to-date, even if upload-triggered updates
    // were missed (e.g. due to a prior decoding error).
    let _ = update_progress_after_upload(pool, user_id).await;
    let missions = repository::get_missions_for_week(pool, user_id, week_start).await?;

    let can_reroll = !missions.iter().any(|m| m.rerolled);

    Ok(WeeklyMissionsResponse {
        week_start,
        missions,
        can_reroll,
    })
}

/// Reroll a single mission (replace with a new one of a different type).
pub async fn reroll_mission(
    pool: &PgPool,
    user_id: Uuid,
    mission_id: Uuid,
) -> Result<WeeklyMission, AppError> {
    let week_start = current_week_start();

    // Validate: mission belongs to this user and current week
    let mission = repository::get_mission_by_id(pool, mission_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if mission.user_id != user_id || mission.week_start != week_start {
        return Err(AppError::NotFound);
    }

    // Validate: reroll not already used this week
    let missions = repository::get_missions_for_week(pool, user_id, week_start).await?;
    if missions.iter().any(|m| m.rerolled) {
        return Err(AppError::BadRequest(
            "Reroll already used this week".to_string(),
        ));
    }

    // Mark the old mission as rerolled
    repository::mark_mission_rerolled(pool, mission_id).await?;
    repository::delete_mission(pool, mission_id).await?;

    // Generate a replacement of a different type
    let existing_types: std::collections::HashSet<String> = missions
        .iter()
        .filter(|m| m.id != mission_id)
        .map(|m| m.mission_type.clone())
        .collect();

    let stats = fetch_weekly_stats(pool, user_id, week_start).await;
    let all_candidates = generate_replacement_candidates(user_id, week_start, &stats);

    let replacement = all_candidates
        .into_iter()
        .find(|m| !existing_types.contains(&m.mission_type))
        .ok_or_else(|| AppError::BadRequest("No replacement mission available".to_string()))?;

    repository::insert_missions(pool, std::slice::from_ref(&replacement)).await?;

    Ok(replacement)
}

/// Build a pool of candidate replacement missions (excluding those already selected by generate_missions).
fn generate_replacement_candidates(
    user_id: Uuid,
    week_start: NaiveDate,
    stats: &UserWeeklyStats,
) -> Vec<WeeklyMission> {
    let now = Utc::now();
    let mut candidates = Vec::new();

    // run_count
    let target_count = ((stats.avg_weekly_runs * 1.1).round() as u32).max(2) as f64;
    candidates.push(WeeklyMission {
        id: Uuid::new_v4(),
        user_id,
        week_start,
        mission_type: "run_count".to_string(),
        title: format!("Run {} times this week", target_count as u32),
        description: format!("Complete {} runs this week", target_count as u32),
        target_value: target_count,
        current_value: 0.0,
        xp_reward: 100,
        completed_at: None,
        rerolled: false,
        created_at: now,
        updated_at: now,
    });

    // longest_single_run
    let target_long = ((stats.avg_weekly_km * 0.6 * 1.05 * 10.0).round() / 10.0).max(3.0);
    candidates.push(WeeklyMission {
        id: Uuid::new_v4(),
        user_id,
        week_start,
        mission_type: "longest_single_run".to_string(),
        title: format!("Go long — {target_long}km or more"),
        description: format!("Complete a single run of at least {target_long}km"),
        target_value: target_long,
        current_value: 0.0,
        xp_reward: 100,
        completed_at: None,
        rerolled: false,
        created_at: now,
        updated_at: now,
    });

    // run_sub_pace
    let target_pace_secs = (stats.avg_pace_secs - 10.0).max(180.0);
    let mins = (target_pace_secs / 60.0).floor() as u32;
    let secs = (target_pace_secs % 60.0).round() as u32;
    let pace_str = format!("{mins}:{secs:02}");
    candidates.push(WeeklyMission {
        id: Uuid::new_v4(),
        user_id,
        week_start,
        mission_type: "run_sub_pace".to_string(),
        title: format!("Run sub {pace_str}/km for 5km+"),
        description: format!("Complete a 5km+ run under {pace_str}/km"),
        target_value: target_pace_secs,
        current_value: f64::MAX,
        xp_reward: 100,
        completed_at: None,
        rerolled: false,
        created_at: now,
        updated_at: now,
    });

    candidates
}

/// Recalculate mission progress for all current-week missions after an upload.
/// Returns summaries of newly-completed missions.
pub async fn update_progress_after_upload(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<CompletedMissionSummary>, AppError> {
    let week_start = current_week_start();
    let missions = repository::get_missions_for_week(pool, user_id, week_start).await?;

    if missions.is_empty() {
        return Ok(vec![]);
    }

    let week_start_dt = week_start.and_hms_opt(0, 0, 0).unwrap();
    let week_end_dt = (week_start + Duration::days(7)).and_hms_opt(0, 0, 0).unwrap();

    // Fetch this week's activity stats in one query.
    // Aggregate SELECT always returns one row (even for empty set), so fetch_one is safe.
    struct WeekStats {
        total_km: f64,
        run_count: i64,
        longest_km: f64,
    }

    let ws_row: (Option<f64>, Option<i64>, Option<f64>) =
        sqlx::query_as::<_, (Option<f64>, Option<i64>, Option<f64>)>(
        r#"
        SELECT
            SUM(distance::FLOAT8) AS total_km,
            COUNT(*) AS run_count,
            MAX(distance::FLOAT8) AS longest_km
        FROM activities
        WHERE user_id = $1 AND date >= $2 AND date < $3
        "#,
    )
    .bind(user_id)
    .bind(week_start_dt)
    .bind(week_end_dt)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        tracing::error!("update_progress week stats query error: {e}");
        AppError::Internal
    })?;

    let ws = WeekStats {
        total_km: ws_row.0.unwrap_or(0.0),
        run_count: ws_row.1.unwrap_or(0),
        longest_km: ws_row.2.unwrap_or(0.0),
    };

    // Best pace this week for runs ≥ 5km (secs/km, lower is better)
    let best_pace_secs: Option<f64> = {
        let pace_raw: Option<Option<f64>> = sqlx::query_scalar(
            r#"
            SELECT MIN(
                CASE
                    WHEN average_pace > 0 THEN
                        (FLOOR(average_pace) * 60.0 + ((average_pace - FLOOR(average_pace)) * 100.0))
                    ELSE NULL
                END
            )
            FROM activities
            WHERE user_id = $1
              AND date >= $2
              AND date < $3
              AND distance >= 5
            "#,
        )
        .bind(user_id)
        .bind(week_start_dt)
        .bind(week_end_dt)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        pace_raw.flatten()
    };

    let mut newly_completed: Vec<CompletedMissionSummary> = Vec::new();

    for mission in &missions {
        let was_completed = mission.completed_at.is_some();

        let (new_value, is_done) = match mission.mission_type.as_str() {
            "run_distance_km" => {
                let v = ws.total_km;
                (v, v >= mission.target_value)
            }
            "run_count" => {
                let v = ws.run_count as f64;
                (v, v >= mission.target_value)
            }
            "beat_last_week_km" => {
                let v = ws.total_km;
                (v, v >= mission.target_value)
            }
            "longest_single_run" => {
                let v = ws.longest_km;
                (v, v >= mission.target_value)
            }
            "run_sub_pace" => {
                // Lower pace = better; completed if best_pace <= target
                let v = best_pace_secs.unwrap_or(f64::MAX);
                let done = best_pace_secs
                    .map(|p| p <= mission.target_value)
                    .unwrap_or(false);
                (v, done)
            }
            "run_on_skipped_day" => {
                // Count runs on the target_value DOW this week
                let target_dow = mission.target_value as i32;
                let ran_on_day: Option<i64> = sqlx::query_scalar(
                    r#"
                    SELECT COUNT(*)
                    FROM activities
                    WHERE user_id = $1
                      AND date >= $2
                      AND date < $3
                      AND CAST(extract(dow FROM date) AS INT) = $4
                    "#,
                )
                .bind(user_id)
                .bind(week_start_dt)
                .bind(week_end_dt)
                .bind(target_dow)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
                .flatten();

                let v = ran_on_day.unwrap_or(0) as f64;
                (v, v >= 1.0)
            }
            _ => continue,
        };

        let completed_at = if is_done && !was_completed {
            Some(Utc::now())
        } else {
            mission.completed_at
        };

        repository::update_mission_progress(pool, mission.id, new_value, completed_at).await?;

        if is_done && !was_completed {
            // Award XP for newly completed mission
            let input = AwardXpInput {
                user_id,
                source_type: "mission".to_string(),
                source_id: Some(mission.id),
                xp_amount: mission.xp_reward,
                description: format!("Mission complete: {}", mission.title),
            };
            if let Err(e) = xp_service::award_xp(pool, input).await {
                tracing::warn!("Failed to award XP for mission {}: {e}", mission.id);
            }

            newly_completed.push(CompletedMissionSummary {
                id: mission.id,
                title: mission.title.clone(),
                xp_reward: mission.xp_reward,
                is_boss: false,
            });
        }
    }

    Ok(newly_completed)
}
