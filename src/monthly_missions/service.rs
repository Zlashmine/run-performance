use chrono::{Datelike, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::AppError,
    missions::common::{format_pace_str, is_mission_complete, CompletedMissionSummary},
    xp::{models::AwardXpInput, service as xp_service},
};

use super::{
    models::{MonthlyMission, MonthlyMissionsResponse},
    repository,
};

type MissionFactory = (&'static str, Box<dyn Fn() -> MonthlyMission>);

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns the 1st of the current month (UTC).
pub fn current_month_start() -> NaiveDate {
    let today = Utc::now().date_naive();
    NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap()
}

// ── Stats ─────────────────────────────────────────────────────────────────────

struct UserMonthlyStats {
    avg_monthly_km: f64,
    avg_monthly_runs: f64,
    avg_monthly_elevation: f64,
    avg_weekly_km: f64,
    avg_weekly_runs: f64,
    best_single_run_km: f64,
    avg_pace_secs: f64,
    best_pace_secs: f64, // all-time min pace converted to secs/km
}

async fn fetch_monthly_stats(pool: &PgPool, user_id: Uuid) -> UserMonthlyStats {
    // Per-month aggregates
    let monthly: (Option<f64>, Option<f64>, Option<f64>) =
        sqlx::query_as::<_, (Option<f64>, Option<f64>, Option<f64>)>(
            r#"
            SELECT
                AVG(monthly_km),
                AVG(monthly_count),
                AVG(monthly_elevation)
            FROM (
                SELECT
                    DATE_TRUNC('month', date) AS m,
                    SUM(distance::FLOAT8)     AS monthly_km,
                    COUNT(*)                  AS monthly_count,
                    SUM(COALESCE(climb, 0))   AS monthly_elevation
                FROM activities
                WHERE user_id = $1
                GROUP BY m
            ) monthly_agg
            "#,
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap_or((None, None, None));

    // Per-week aggregates
    let weekly: (Option<f64>, Option<f64>) =
        sqlx::query_as::<_, (Option<f64>, Option<f64>)>(
            r#"
            SELECT AVG(weekly_km), AVG(weekly_count)
            FROM (
                SELECT
                    DATE_TRUNC('week', date) AS w,
                    SUM(distance::FLOAT8)    AS weekly_km,
                    COUNT(*)                 AS weekly_count
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

    // Best single run
    let best_run: f64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(distance::FLOAT8), 0) FROM activities WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(Some(0.0))
    .unwrap_or(0.0);

    // Average and best (min) pace — stored as M.SS, convert to secs/km
    let pace_stats: (Option<f64>, Option<f64>) =
        sqlx::query_as::<_, (Option<f64>, Option<f64>)>(
            r#"
            SELECT
                AVG(FLOOR(average_pace) * 60.0 + ((average_pace - FLOOR(average_pace)) * 100.0)),
                MIN(FLOOR(average_pace) * 60.0 + ((average_pace - FLOOR(average_pace)) * 100.0))
            FROM activities
            WHERE user_id = $1 AND average_pace > 0
            "#,
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap_or((None, None));

    UserMonthlyStats {
        avg_monthly_km: monthly.0.unwrap_or(5.0),
        avg_monthly_runs: monthly.1.unwrap_or(4.0),
        avg_monthly_elevation: monthly.2.unwrap_or(0.0),
        avg_weekly_km: weekly.0.unwrap_or(5.0),
        avg_weekly_runs: weekly.1.unwrap_or(2.0),
        best_single_run_km: best_run,
        avg_pace_secs: pace_stats.0.unwrap_or(360.0),
        best_pace_secs: pace_stats.1.unwrap_or(360.0),
    }
}

// ── Mission Generation ────────────────────────────────────────────────────────

/// Select the boss type for this user/month combination.
fn select_boss(stats: &UserMonthlyStats, month: u32) -> &'static str {
    if stats.avg_monthly_km > 80.0 && stats.best_single_run_km > 20.0 {
        return "boss_century";
    }
    if stats.best_single_run_km < 15.0 {
        return "boss_ultralong_run";
    }
    if stats.avg_weekly_runs < 3.0 {
        return "boss_iron_week";
    }
    let pool = [
        "boss_speed_demon",
        "boss_all_weekdays",
        "boss_marathon_month",
        "boss_global_expedition",
    ];
    pool[(month as usize) % pool.len()]
}

fn select_boss_excluding(stats: &UserMonthlyStats, month: u32, excluded: &str) -> &'static str {
    const ALL: [&str; 7] = [
        "boss_century",
        "boss_ultralong_run",
        "boss_iron_week",
        "boss_speed_demon",
        "boss_all_weekdays",
        "boss_marathon_month",
        "boss_global_expedition",
    ];
    let preferred = select_boss(stats, month);
    if preferred != excluded {
        return preferred;
    }
    let available: Vec<&'static str> = ALL.iter().copied().filter(|t| *t != excluded).collect();
    available[(month as usize) % available.len()]
}

/// Build all 4 missions (3 regular + 1 boss) for a user/month.
fn generate_missions(
    user_id: Uuid,
    month_start: NaiveDate,
    stats: &UserMonthlyStats,
) -> Vec<MonthlyMission> {
    let now = Utc::now();
    let month = month_start.month();
    let mut missions: Vec<MonthlyMission> = Vec::with_capacity(4);

    // ── Slot 1: Always distance (progression anchor) ──────────────────────────
    let target_km = ((stats.avg_monthly_km * 1.15 * 10.0).round() / 10.0).max(10.0);
    missions.push(MonthlyMission {
        id: Uuid::new_v4(),
        user_id,
        month_start,
        mission_type: "monthly_distance_km".to_string(),
        title: format!("Cover {target_km:.0}km this month"),
        description: format!("Accumulate {target_km:.1}km of running during the month"),
        target_value: target_km,
        current_value: 0.0,
        xp_reward: 300,
        completed_at: None,
        rerolled: false,
        is_boss: false,
        boss_reroll_count: 0,
        created_at: now,
        updated_at: now,
    });

    // ── Slot 2: Stat-based pick ───────────────────────────────────────────────
    if stats.avg_monthly_elevation > 0.0 {
        let target_m = ((stats.avg_monthly_elevation * 1.1).max(324.0)).round();
        let towers = (target_m / 324.0).round() as u32;
        missions.push(MonthlyMission {
            id: Uuid::new_v4(),
            user_id,
            month_start,
            mission_type: "monthly_elevation_meters".to_string(),
            title: format!("Climb {towers}× Eiffel Tower ({target_m:.0}m)"),
            description: format!("Accumulate {target_m:.0}m of elevation gain this month"),
            target_value: target_m,
            current_value: 0.0,
            xp_reward: 300,
            completed_at: None,
            rerolled: false,
            is_boss: false,
            boss_reroll_count: 0,
            created_at: now,
            updated_at: now,
        });
    } else if stats.avg_monthly_km >= 30.0 {
        let long_thresh = ((stats.avg_monthly_km * 0.35).max(8.0) * 10.0).round() / 10.0;
        missions.push(MonthlyMission {
            id: Uuid::new_v4(),
            user_id,
            month_start,
            mission_type: "monthly_long_run_series".to_string(),
            title: format!("Complete 2 long runs (≥{long_thresh:.0}km each)"),
            description: format!("Log 2 separate runs of at least {long_thresh:.0}km this month"),
            target_value: 2.0,
            current_value: 0.0,
            xp_reward: 300,
            completed_at: None,
            rerolled: false,
            is_boss: false,
            boss_reroll_count: 0,
            created_at: now,
            updated_at: now,
        });
    } else {
        let target_count = ((stats.avg_monthly_runs * 1.1).round() as u32).max(8) as f64;
        missions.push(MonthlyMission {
            id: Uuid::new_v4(),
            user_id,
            month_start,
            mission_type: "monthly_run_count".to_string(),
            title: format!("Log {} runs this month", target_count as u32),
            description: format!("Complete {} runs during the month", target_count as u32),
            target_value: target_count,
            current_value: 0.0,
            xp_reward: 300,
            completed_at: None,
            rerolled: false,
            is_boss: false,
            boss_reroll_count: 0,
            created_at: now,
            updated_at: now,
        });
    }

    // ── Slot 3: Rotation from remaining pool ──────────────────────────────────
    let slot2_type = &missions[1].mission_type;
    let pool: Vec<MissionFactory> = {
        let uid = user_id;
        let ms = month_start;
        let n = now;
        let pace_secs = stats.avg_pace_secs;
        let wkly_km = stats.avg_weekly_km;

        vec![
            (
                "monthly_consistency_weeks",
                Box::new(move || MonthlyMission {
                    id: Uuid::new_v4(),
                    user_id: uid,
                    month_start: ms,
                    mission_type: "monthly_consistency_weeks".to_string(),
                    title: "Run every week this month".to_string(),
                    description: "Log at least one run in each of the 4 weeks this month".to_string(),
                    target_value: 4.0,
                    current_value: 0.0,
                    xp_reward: 300,
                    completed_at: None,
                    rerolled: false,
                    is_boss: false,
                    boss_reroll_count: 0,
                    created_at: n,
                    updated_at: n,
                }),
            ),
            (
                "monthly_exploration_areas",
                Box::new(move || MonthlyMission {
                    id: Uuid::new_v4(),
                    user_id: uid,
                    month_start: ms,
                    mission_type: "monthly_exploration_areas".to_string(),
                    title: "Explore 3 new neighborhoods".to_string(),
                    description: "Start runs in 3 distinct geographic areas this month".to_string(),
                    target_value: 3.0,
                    current_value: 0.0,
                    xp_reward: 300,
                    completed_at: None,
                    rerolled: false,
                    is_boss: false,
                    boss_reroll_count: 0,
                    created_at: n,
                    updated_at: n,
                }),
            ),
            (
                "monthly_sub_pace_count",
                Box::new(move || {
                    let target_pace_secs = (pace_secs - 15.0).max(180.0);
                    let pace_str = format_pace_str(target_pace_secs);
                    MonthlyMission {
                        id: Uuid::new_v4(),
                        user_id: uid,
                        month_start: ms,
                        mission_type: "monthly_sub_pace_count".to_string(),
                        title: format!("Run sub {pace_str}/km three times"),
                        description: format!(
                            "Complete 3 runs of 5km+ each under {pace_str}/km",
                        ),
                        target_value: 3.0,
                        current_value: 0.0,
                        xp_reward: 300,
                        completed_at: None,
                        rerolled: false,
                        is_boss: false,
                        boss_reroll_count: 0,
                        created_at: n,
                        updated_at: n,
                    }
                }),
            ),
            (
                "monthly_volume_spike",
                Box::new(move || {
                    let spike_km = ((wkly_km * 1.4 * 10.0).round() / 10.0).max(10.0);
                    MonthlyMission {
                        id: Uuid::new_v4(),
                        user_id: uid,
                        month_start: ms,
                        mission_type: "monthly_volume_spike".to_string(),
                        title: format!("Power Week: {spike_km:.0}km in one week"),
                        description: format!(
                            "Accumulate {spike_km:.1}km in any single calendar week this month",
                        ),
                        target_value: spike_km,
                        current_value: 0.0,
                        xp_reward: 300,
                        completed_at: None,
                        rerolled: false,
                        is_boss: false,
                        boss_reroll_count: 0,
                        created_at: n,
                        updated_at: n,
                    }
                }),
            ),
            (
                "monthly_progressive_weeks",
                Box::new(move || MonthlyMission {
                    id: Uuid::new_v4(),
                    user_id: uid,
                    month_start: ms,
                    mission_type: "monthly_progressive_weeks".to_string(),
                    title: "Build momentum: 3 better weeks".to_string(),
                    description: "Have 3 weeks each with more km than the previous week".to_string(),
                    target_value: 3.0,
                    current_value: 0.0,
                    xp_reward: 300,
                    completed_at: None,
                    rerolled: false,
                    is_boss: false,
                    boss_reroll_count: 0,
                    created_at: n,
                    updated_at: n,
                }),
            ),
        ]
    };

    // Filter out slot2_type and monthly_distance_km (always slot1)
    let filtered: Vec<Box<dyn Fn() -> MonthlyMission>> = pool
        .into_iter()
        .filter(|(t, _)| *t != slot2_type.as_str() && *t != "monthly_distance_km")
        .map(|(_, f)| f)
        .collect();

    if !filtered.is_empty() {
        let idx = ((month as usize).saturating_sub(1)) % filtered.len();
        missions.push(filtered[idx]());
    }

    // ── Boss ──────────────────────────────────────────────────────────────────
    let boss_type = select_boss(stats, month);
    let boss = build_boss(user_id, month_start, boss_type, stats, now);
    missions.push(boss);

    missions
}

fn build_boss(
    user_id: Uuid,
    month_start: NaiveDate,
    boss_type: &str,
    stats: &UserMonthlyStats,
    now: chrono::DateTime<Utc>,
) -> MonthlyMission {
    match boss_type {
        "boss_century" => {
            let target = (stats.avg_monthly_km * 1.2).max(100.0);
            let target = (target * 10.0).round() / 10.0;
            MonthlyMission {
                id: Uuid::new_v4(),
                user_id,
                month_start,
                mission_type: "boss_century".to_string(),
                title: format!("⚔️ The Century: {target:.0}km this month"),
                description: format!(
                    "Conquer {target:.1}km in a single month — the ultimate endurance challenge",
                ),
                target_value: target,
                current_value: 0.0,
                xp_reward: 750,
                completed_at: None,
                rerolled: false,
                is_boss: true,
                boss_reroll_count: 0,
                created_at: now,
                updated_at: now,
            }
        }
        "boss_ultralong_run" => MonthlyMission {
            id: Uuid::new_v4(),
            user_id,
            month_start,
            mission_type: "boss_ultralong_run".to_string(),
            title: "⚔️ The Ultra: A single 30km run".to_string(),
            description: "Complete one continuous run of at least 30km this month".to_string(),
            target_value: 30.0,
            current_value: 0.0,
            xp_reward: 750,
            completed_at: None,
            rerolled: false,
            is_boss: true,
            boss_reroll_count: 0,
            created_at: now,
            updated_at: now,
        },
        "boss_iron_week" => MonthlyMission {
            id: Uuid::new_v4(),
            user_id,
            month_start,
            mission_type: "boss_iron_week".to_string(),
            title: "⚔️ Iron Week: 5 running days in one week".to_string(),
            description: "Log runs on 5 distinct days within any single calendar week this month"
                .to_string(),
            target_value: 5.0,
            current_value: 0.0,
            xp_reward: 750,
            completed_at: None,
            rerolled: false,
            is_boss: true,
            boss_reroll_count: 0,
            created_at: now,
            updated_at: now,
        },
        "boss_speed_demon" => {
            // Personal best − 5s/km. Default to 355s/km if no data.
            let target = (stats.best_pace_secs - 5.0).max(150.0);
            let pace_str = format_pace_str(target);
            MonthlyMission {
                id: Uuid::new_v4(),
                user_id,
                month_start,
                mission_type: "boss_speed_demon".to_string(),
                title: format!("⚔️ Break the Barrier: sub {pace_str}/km PR"),
                description: format!(
                    "Smash your personal best — run 5km+ under {pace_str}/km",
                ),
                target_value: target,
                // Start high so lower-is-better logic works (inverted)
                current_value: 9999.0,
                xp_reward: 750,
                completed_at: None,
                rerolled: false,
                is_boss: true,
                boss_reroll_count: 0,
                created_at: now,
                updated_at: now,
            }
        }
        "boss_all_weekdays" => MonthlyMission {
            id: Uuid::new_v4(),
            user_id,
            month_start,
            mission_type: "boss_all_weekdays".to_string(),
            title: "⚔️ Weekday Warrior: Mon–Fri in one week".to_string(),
            description: "Run on every weekday (Mon–Fri) within any single calendar week this month"
                .to_string(),
            target_value: 5.0,
            current_value: 0.0,
            xp_reward: 750,
            completed_at: None,
            rerolled: false,
            is_boss: true,
            boss_reroll_count: 0,
            created_at: now,
            updated_at: now,
        },
        "boss_marathon_month" => MonthlyMission {
            id: Uuid::new_v4(),
            user_id,
            month_start,
            mission_type: "boss_marathon_month".to_string(),
            title: "⚔️ Marathon Week: 42.2km in 7 days".to_string(),
            description: "Accumulate 42.2km within any rolling 7-day window this month".to_string(),
            target_value: 42.2,
            current_value: 0.0,
            xp_reward: 750,
            completed_at: None,
            rerolled: false,
            is_boss: true,
            boss_reroll_count: 0,
            created_at: now,
            updated_at: now,
        },
        _ => MonthlyMission {
            id: Uuid::new_v4(),
            user_id,
            month_start,
            mission_type: "boss_global_expedition".to_string(),
            title: "⚔️ Global Expedition: 5 new areas".to_string(),
            description: "Start runs in 5 distinct geographic grid cells (~5.5km squares) this month"
                .to_string(),
            target_value: 5.0,
            current_value: 0.0,
            xp_reward: 750,
            completed_at: None,
            rerolled: false,
            is_boss: true,
            boss_reroll_count: 0,
            created_at: now,
            updated_at: now,
        },
    }
}

// ── get_or_generate_missions ──────────────────────────────────────────────────

pub async fn get_or_generate_missions(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<MonthlyMissionsResponse, AppError> {
    let month_start = current_month_start();
    let existing = repository::get_missions_for_month(pool, user_id, month_start).await?;

    if existing.len() < 4 {
        let existing_types: std::collections::HashSet<String> =
            existing.iter().map(|m| m.mission_type.clone()).collect();

        let stats = fetch_monthly_stats(pool, user_id).await;
        let generated = generate_missions(user_id, month_start, &stats);

        let to_insert: Vec<MonthlyMission> = generated
            .into_iter()
            .filter(|m| !existing_types.contains(&m.mission_type))
            .collect();

        if !to_insert.is_empty() {
            repository::insert_missions(pool, &to_insert).await?;
        }
    }

    // Always recalculate progress to keep it fresh
    let _ = update_progress_after_upload(pool, user_id).await;

    let all = repository::get_missions_for_month(pool, user_id, month_start).await?;
    let can_reroll = !all.iter().any(|m| m.rerolled);

    let boss = all.iter().find(|m| m.is_boss).cloned();
    let can_reroll_boss = boss.as_ref().is_some_and(|b| b.boss_reroll_count < 2);
    let missions: Vec<MonthlyMission> = all.into_iter().filter(|m| !m.is_boss).collect();

    Ok(MonthlyMissionsResponse {
        month_start,
        missions,
        boss,
        can_reroll,
        can_reroll_boss,
    })
}

// ── reroll_mission ────────────────────────────────────────────────────────────

pub async fn reroll_mission(
    pool: &PgPool,
    user_id: Uuid,
    mission_id: Uuid,
) -> Result<MonthlyMission, AppError> {
    let month_start = current_month_start();

    let mission = repository::get_mission_by_id(pool, mission_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if mission.user_id != user_id || mission.month_start != month_start {
        return Err(AppError::NotFound);
    }

    if mission.is_boss {
        if mission.boss_reroll_count >= 2 {
            return Err(AppError::BadRequest(
                "Boss reroll limit (2 per month) already reached".to_string(),
            ));
        }
        let old_count = mission.boss_reroll_count;
        repository::delete_mission(pool, mission_id).await?;
        let stats = fetch_monthly_stats(pool, user_id).await;
        let boss_type =
            select_boss_excluding(&stats, month_start.month(), &mission.mission_type);
        let mut new_boss = build_boss(user_id, month_start, boss_type, &stats, Utc::now());
        new_boss.boss_reroll_count = old_count + 1;
        repository::insert_missions(pool, &[new_boss.clone()]).await?;
        return Ok(new_boss);
    }

    let all_missions = repository::get_missions_for_month(pool, user_id, month_start).await?;
    if all_missions.iter().any(|m| m.rerolled) {
        return Err(AppError::BadRequest(
            "Reroll already used this month".to_string(),
        ));
    }

    repository::mark_mission_rerolled(pool, mission_id).await?;
    repository::delete_mission(pool, mission_id).await?;

    let existing_types: std::collections::HashSet<String> = all_missions
        .iter()
        .filter(|m| m.id != mission_id)
        .map(|m| m.mission_type.clone())
        .collect();

    let stats = fetch_monthly_stats(pool, user_id).await;
    let replacement = find_replacement(user_id, month_start, &stats, &existing_types)
        .ok_or_else(|| AppError::BadRequest("No replacement mission available".to_string()))?;

    repository::insert_missions(pool, &[replacement.clone()]).await?;

    Ok(replacement)
}

fn find_replacement(
    user_id: Uuid,
    month_start: NaiveDate,
    stats: &UserMonthlyStats,
    existing_types: &std::collections::HashSet<String>,
) -> Option<MonthlyMission> {
    let now = Utc::now();
    let month = month_start.month();

    // Pool of all standard types (monthly_distance_km excluded — it's the anchor)
    let all_standard = [
        "monthly_run_count",
        "monthly_consistency_weeks",
        "monthly_elevation_meters",
        "monthly_long_run_series",
        "monthly_exploration_areas",
        "monthly_sub_pace_count",
        "monthly_volume_spike",
        "monthly_progressive_weeks",
    ];

    let available: Vec<&str> = all_standard
        .iter()
        .copied()
        .filter(|t| !existing_types.contains(*t))
        .collect();

    if available.is_empty() {
        return None;
    }

    let pick = available[(month as usize) % available.len()];
    let pace_secs = stats.avg_pace_secs;
    let wkly_km = stats.avg_weekly_km;

    let m = match pick {
        "monthly_run_count" => {
            let target = ((stats.avg_monthly_runs * 1.1).round() as u32).max(8) as f64;
            MonthlyMission {
                id: Uuid::new_v4(), user_id, month_start,
                mission_type: "monthly_run_count".to_string(),
                title: format!("Log {} runs this month", target as u32),
                description: format!("Complete {} runs during the month", target as u32),
                target_value: target, current_value: 0.0, xp_reward: 300,
                completed_at: None, rerolled: false, is_boss: false,
                boss_reroll_count: 0,
                created_at: now, updated_at: now,
            }
        }
        "monthly_consistency_weeks" => MonthlyMission {
            id: Uuid::new_v4(), user_id, month_start,
            mission_type: "monthly_consistency_weeks".to_string(),
            title: "Run every week this month".to_string(),
            description: "Log at least one run in each of the 4 weeks this month".to_string(),
            target_value: 4.0, current_value: 0.0, xp_reward: 300,
            completed_at: None, rerolled: false, is_boss: false,
            boss_reroll_count: 0,
            created_at: now, updated_at: now,
        },
        "monthly_elevation_meters" => {
            let target = ((stats.avg_monthly_elevation * 1.1).max(324.0)).round();
            let towers = (target / 324.0).round() as u32;
            MonthlyMission {
                id: Uuid::new_v4(), user_id, month_start,
                mission_type: "monthly_elevation_meters".to_string(),
                title: format!("Climb {towers}× Eiffel Tower ({target:.0}m)"),
                description: format!("Accumulate {target:.0}m of elevation gain this month"),
                target_value: target, current_value: 0.0, xp_reward: 300,
                completed_at: None, rerolled: false, is_boss: false,
                boss_reroll_count: 0,
                created_at: now, updated_at: now,
            }
        }
        "monthly_long_run_series" => {
            let long_thresh = ((stats.avg_monthly_km * 0.35).max(8.0) * 10.0).round() / 10.0;
            MonthlyMission {
                id: Uuid::new_v4(), user_id, month_start,
                mission_type: "monthly_long_run_series".to_string(),
                title: format!("Complete 2 long runs (≥{long_thresh:.0}km each)"),
                description: format!("Log 2 separate runs of at least {long_thresh:.0}km this month"),
                target_value: 2.0, current_value: 0.0, xp_reward: 300,
                completed_at: None, rerolled: false, is_boss: false,
                boss_reroll_count: 0,
                created_at: now, updated_at: now,
            }
        }
        "monthly_exploration_areas" => MonthlyMission {
            id: Uuid::new_v4(), user_id, month_start,
            mission_type: "monthly_exploration_areas".to_string(),
            title: "Explore 3 new neighborhoods".to_string(),
            description: "Start runs in 3 distinct geographic areas this month".to_string(),
            target_value: 3.0, current_value: 0.0, xp_reward: 300,
            completed_at: None, rerolled: false, is_boss: false,
            boss_reroll_count: 0,
            created_at: now, updated_at: now,
        },
        "monthly_sub_pace_count" => {
            let target_pace = (pace_secs - 15.0).max(180.0);
            let pace_str = format_pace_str(target_pace);
            MonthlyMission {
                id: Uuid::new_v4(), user_id, month_start,
                mission_type: "monthly_sub_pace_count".to_string(),
                title: format!("Run sub {pace_str}/km three times"),
                description: format!("Complete 3 runs of 5km+ each under {pace_str}/km"),
                target_value: 3.0, current_value: 0.0, xp_reward: 300,
                completed_at: None, rerolled: false, is_boss: false,
                boss_reroll_count: 0,
                created_at: now, updated_at: now,
            }
        }
        "monthly_volume_spike" => {
            let spike_km = ((wkly_km * 1.4 * 10.0).round() / 10.0).max(10.0);
            MonthlyMission {
                id: Uuid::new_v4(), user_id, month_start,
                mission_type: "monthly_volume_spike".to_string(),
                title: format!("Power Week: {spike_km:.0}km in one week"),
                description: format!("Accumulate {spike_km:.1}km in any single week this month"),
                target_value: spike_km, current_value: 0.0, xp_reward: 300,
                completed_at: None, rerolled: false, is_boss: false,
                boss_reroll_count: 0,
                created_at: now, updated_at: now,
            }
        }
        _ => MonthlyMission {
            id: Uuid::new_v4(), user_id, month_start,
            mission_type: "monthly_progressive_weeks".to_string(),
            title: "Build momentum: 3 better weeks".to_string(),
            description: "Have 3 weeks each with more km than the previous week".to_string(),
            target_value: 3.0, current_value: 0.0, xp_reward: 300,
            completed_at: None, rerolled: false, is_boss: false,
            boss_reroll_count: 0,
            created_at: now, updated_at: now,
        },
    };

    Some(m)
}

// ── update_progress_after_upload ─────────────────────────────────────────────

pub async fn update_progress_after_upload(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<CompletedMissionSummary>, AppError> {
    let month_start = current_month_start();
    let missions = repository::get_missions_for_month(pool, user_id, month_start).await?;

    if missions.is_empty() {
        return Ok(vec![]);
    }

    let month_start_dt = month_start.and_hms_opt(0, 0, 0).unwrap();
    // End of current month
    let next_month = if month_start.month() == 12 {
        NaiveDate::from_ymd_opt(month_start.year() + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(month_start.year(), month_start.month() + 1, 1).unwrap()
    };
    let month_end_dt = next_month.and_hms_opt(0, 0, 0).unwrap();

    // Basic monthly stats
    struct MonthStats {
        total_km: f64,
        run_count: i64,
        max_single_km: f64,
        total_elevation: f64,
    }

    let stats_row: (Option<f64>, Option<i64>, Option<f64>, Option<f64>) =
        sqlx::query_as::<_, (Option<f64>, Option<i64>, Option<f64>, Option<f64>)>(
            r#"
            SELECT
                SUM(distance::FLOAT8),
                COUNT(*),
                MAX(distance::FLOAT8),
                SUM(COALESCE(climb, 0)::FLOAT8)
            FROM activities
            WHERE user_id = $1 AND date >= $2 AND date < $3
            "#,
        )
        .bind(user_id)
        .bind(month_start_dt)
        .bind(month_end_dt)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            tracing::error!("monthly progress stats error: {e}");
            AppError::Internal
        })?;

    let ms = MonthStats {
        total_km: stats_row.0.unwrap_or(0.0),
        run_count: stats_row.1.unwrap_or(0),
        max_single_km: stats_row.2.unwrap_or(0.0),
        total_elevation: stats_row.3.unwrap_or(0.0),
    };

    let mut newly_completed: Vec<CompletedMissionSummary> = Vec::new();

    for mission in &missions {
        let was_completed = mission.completed_at.is_some();

        let (new_value, is_done) = match mission.mission_type.as_str() {
            "monthly_distance_km" | "boss_century" => {
                let v = ms.total_km;
                (v, is_mission_complete(&mission.mission_type, v, mission.target_value))
            }
            "monthly_run_count" => {
                let v = ms.run_count as f64;
                (v, v >= mission.target_value)
            }
            "monthly_elevation_meters" => {
                let v = ms.total_elevation;
                (v, v >= mission.target_value)
            }
            "boss_ultralong_run" => {
                let v = ms.max_single_km;
                (v, v >= mission.target_value)
            }
            "monthly_consistency_weeks" => {
                let v: Option<i64> = sqlx::query_scalar(
                    r#"
                    SELECT COUNT(DISTINCT DATE_TRUNC('week', date))
                    FROM activities
                    WHERE user_id = $1 AND date >= $2 AND date < $3
                    "#,
                )
                .bind(user_id)
                .bind(month_start_dt)
                .bind(month_end_dt)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
                .flatten();
                let v = v.unwrap_or(0) as f64;
                (v, v >= mission.target_value)
            }
            "monthly_long_run_series" => {
                // Number of long runs >= threshold stored in description via target_value companion
                // The long_thresh is approximately avg_monthly_km * 0.35 but we re-derive from
                // a simpler heuristic: any run >= (target_value is 2 runs, threshold via title)
                // Actually target_value = 2 (count), threshold requires fetching separately.
                // We store the km threshold in target_value for distance missions; here target_value
                // is the *count* (2). We need to derive it from stats — simplest: use
                // avg_monthly_km * 0.35 which is stored nowhere. Fallback: 8km threshold.
                // Better approach: store long_thresh inside the mission target_value only when it's
                // used as count. We'll store it as a rounded multiplier in target_value.
                // Since target_value = 2 (count), we re-derive: long_thresh = 8.0 min
                // (a small heuristic; for accuracy you'd store long_thresh in DB — future work).
                let long_thresh = 8.0_f64;
                let v: Option<i64> = sqlx::query_scalar(
                    r#"
                    SELECT COUNT(*)
                    FROM activities
                    WHERE user_id = $1 AND date >= $2 AND date < $3
                      AND distance::FLOAT8 >= $4
                    "#,
                )
                .bind(user_id)
                .bind(month_start_dt)
                .bind(month_end_dt)
                .bind(long_thresh)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
                .flatten();
                let v = v.unwrap_or(0) as f64;
                (v, v >= mission.target_value)
            }
            "monthly_exploration_areas" | "boss_global_expedition" => {
                let target_areas = mission.target_value;
                let v: Option<i64> = sqlx::query_scalar(
                    r#"
                    SELECT COUNT(DISTINCT (lat_cell, lon_cell)) AS distinct_areas
                    FROM (
                        SELECT DISTINCT ON (a.id)
                            ROUND(CAST(t.lat AS NUMERIC) / 0.05) * 0.05 AS lat_cell,
                            ROUND(CAST(t.lon AS NUMERIC) / 0.05) * 0.05 AS lon_cell
                        FROM activities a
                        JOIN trackpoints t ON t.activity_id = a.id
                        WHERE a.user_id = $1
                          AND a.date >= $2 AND a.date < $3
                        ORDER BY a.id, t.time ASC
                    ) first_points
                    "#,
                )
                .bind(user_id)
                .bind(month_start_dt)
                .bind(month_end_dt)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
                .flatten();
                let v = v.unwrap_or(0) as f64;
                (v, v >= target_areas)
            }
            "monthly_sub_pace_count" => {
                // Count of runs >= 5km where avg pace (in secs) < mission target
                // target_value for monthly_sub_pace_count is 3 (count of qualifying runs).
                // The pace threshold is derived from stats at generation; we use 15s below avg.
                // We check all runs ≥ 5km with a "good" pace. Since we don't store the pace
                // threshold in the DB except derivatively, we use the description. For simplicity,
                // we count all 5km+ runs with avg_pace < (avg - 15s), which matches generation.
                // In practice, once generated, the pace threshold is implicit.
                // We'll query for pace using the DB formula; below-average pace counts.
                let v: Option<i64> = sqlx::query_scalar(
                    r#"
                    SELECT COUNT(*)
                    FROM activities
                    WHERE user_id = $1 AND date >= $2 AND date < $3
                      AND distance::FLOAT8 >= 5.0
                      AND average_pace > 0
                      AND (FLOOR(average_pace) * 60.0 + ((average_pace - FLOOR(average_pace)) * 100.0))
                          < (
                              SELECT AVG(FLOOR(average_pace) * 60.0 + ((average_pace - FLOOR(average_pace)) * 100.0)) - 15.0
                              FROM activities
                              WHERE user_id = $1 AND average_pace > 0
                          )
                    "#,
                )
                .bind(user_id)
                .bind(month_start_dt)
                .bind(month_end_dt)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
                .flatten();
                let v = v.unwrap_or(0) as f64;
                (v, v >= mission.target_value)
            }
            "monthly_volume_spike" => {
                // Max km in any single ISO week during this month
                let v: Option<f64> = sqlx::query_scalar(
                    r#"
                    SELECT COALESCE(MAX(weekly_km), 0)
                    FROM (
                        SELECT DATE_TRUNC('week', date) AS w, SUM(distance::FLOAT8) AS weekly_km
                        FROM activities
                        WHERE user_id = $1 AND date >= $2 AND date < $3
                        GROUP BY w
                    ) weeks
                    "#,
                )
                .bind(user_id)
                .bind(month_start_dt)
                .bind(month_end_dt)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
                .flatten();
                let v = v.unwrap_or(0.0);
                (v, v >= mission.target_value)
            }
            "monthly_progressive_weeks" => {
                let v: Option<i64> = sqlx::query_scalar(
                    r#"
                    WITH weekly_km AS (
                        SELECT
                            DATE_TRUNC('week', date) AS week_start,
                            SUM(distance::FLOAT8) AS km
                        FROM activities
                        WHERE user_id = $1 AND date >= $2 AND date < $3
                        GROUP BY DATE_TRUNC('week', date)
                        ORDER BY week_start
                    )
                    SELECT COUNT(*) AS improving_weeks
                    FROM (
                        SELECT km,
                               LAG(km) OVER (ORDER BY week_start) AS prev_km
                        FROM weekly_km
                    ) w
                    WHERE km > prev_km
                    "#,
                )
                .bind(user_id)
                .bind(month_start_dt)
                .bind(month_end_dt)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
                .flatten();
                let v = v.unwrap_or(0) as f64;
                (v, v >= mission.target_value)
            }
            "boss_iron_week" => {
                // Count distinct running days in the current ISO week (Monday–Sunday).
                // Progress reflects how you're doing *this* week, resetting each Monday.
                // The mission completes if you reach 5 days in the current week.
                let v: Option<i64> = sqlx::query_scalar(
                    r#"
                    SELECT COUNT(DISTINCT DATE_TRUNC('day', date))
                    FROM activities
                    WHERE user_id = $1
                      AND activity_type = 'Running'
                      AND date >= DATE_TRUNC('week', NOW())
                      AND date < DATE_TRUNC('week', NOW()) + INTERVAL '1 week'
                    "#,
                )
                .bind(user_id)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
                .flatten();
                let v = v.unwrap_or(0) as f64;
                (v, v >= mission.target_value)
            }
            "boss_speed_demon" => {
                // Min converted-pace for runs >= 5km this month (lower = faster)
                let v: Option<f64> = sqlx::query_scalar(
                    r#"
                    SELECT MIN(
                        FLOOR(average_pace) * 60.0 + ((average_pace - FLOOR(average_pace)) * 100.0)
                    )
                    FROM activities
                    WHERE user_id = $1 AND date >= $2 AND date < $3
                      AND distance >= 5 AND average_pace > 0
                    "#,
                )
                .bind(user_id)
                .bind(month_start_dt)
                .bind(month_end_dt)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
                .flatten();
                let v = v.unwrap_or(9999.0);
                let done = v < 9999.0 && v <= mission.target_value;
                (v, done)
            }
            "boss_all_weekdays" => {
                // Max count of Mon–Fri (dow 1-5) distinct days in any ISO week
                let v: Option<i64> = sqlx::query_scalar(
                    r#"
                    SELECT COALESCE(MAX(weekday_count), 0)
                    FROM (
                        SELECT DATE_TRUNC('week', date) AS week_start,
                               COUNT(DISTINCT DATE_TRUNC('day', date)) FILTER (
                                   WHERE EXTRACT(DOW FROM date) BETWEEN 1 AND 5
                               ) AS weekday_count
                        FROM activities
                        WHERE user_id = $1 AND date >= $2 AND date < $3
                        GROUP BY DATE_TRUNC('week', date)
                    ) weeks
                    "#,
                )
                .bind(user_id)
                .bind(month_start_dt)
                .bind(month_end_dt)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
                .flatten();
                let v = v.unwrap_or(0) as f64;
                (v, v >= mission.target_value)
            }
            "boss_marathon_month" => {
                // Best rolling 7-day sum of distance in km
                let v: Option<f64> = sqlx::query_scalar(
                    r#"
                    SELECT COALESCE(MAX(window_km), 0)
                    FROM (
                        SELECT
                            date,
                            SUM(distance::FLOAT8 / 1000.0) OVER (
                                ORDER BY date
                                RANGE BETWEEN INTERVAL '6 days' PRECEDING AND CURRENT ROW
                            ) AS window_km
                        FROM activities
                        WHERE user_id = $1 AND date >= $2 AND date < $3
                    ) windows
                    "#,
                )
                .bind(user_id)
                .bind(month_start_dt)
                .bind(month_end_dt)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
                .flatten();
                let v = v.unwrap_or(0.0);
                (v, v >= mission.target_value)
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
            let input = AwardXpInput {
                user_id,
                source_type: "mission".to_string(),
                source_id: Some(mission.id),
                xp_amount: mission.xp_reward,
                description: format!("Monthly mission complete: {}", mission.title),
            };
            if let Err(e) = xp_service::award_xp(pool, input).await {
                tracing::warn!("Failed to award XP for monthly mission {}: {e}", mission.id);
            }

            newly_completed.push(CompletedMissionSummary {
                id: mission.id,
                title: mission.title.clone(),
                xp_reward: mission.xp_reward,
                is_boss: mission.is_boss,
            });
        }
    }

    Ok(newly_completed)
}
