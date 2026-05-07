/// Achievement evaluation rules — pure logic, no DB access.
use std::collections::HashSet;

use chrono::{Datelike, DateTime, Timelike, Utc};
use uuid::Uuid;

pub struct CheckContext {
    #[allow(dead_code)]
    pub user_id: Uuid,
    #[allow(dead_code)]
    pub activity_id: Uuid,
    pub activity_start: DateTime<Utc>,
    /// Distance of THIS activity in metres.
    pub activity_distance_m: f64,
    /// Average pace of THIS activity in min/km (as stored in DB).
    pub activity_pace_min_per_km: f64,
    /// Total runs including this one.
    pub total_runs: i64,
    /// Total distance in metres across all runs including this one.
    pub total_distance_m: f64,
    /// Consecutive-days streak ending at today, including this run.
    pub current_streak: i32,
    /// Recent activities' average_pace values (min/km), last 10, newest first.
    pub recent_paces: Vec<f64>,
    /// Already-unlocked achievement slugs for this user.
    pub already_unlocked: HashSet<String>,
    /// Distinct (year, month) tuples for all runs including this one.
    pub months_with_runs: HashSet<(i32, u32)>,
    /// Number of PRs set so far (including this run).
    pub pr_count: i64,
    /// Number of runs on Mondays.
    pub monday_run_count: i64,
    /// Whether there was a 30+ day gap before this run.
    pub had_long_gap: bool,
}

pub fn evaluate_all(ctx: &CheckContext) -> Vec<&'static str> {
    let checks: &[fn(&CheckContext) -> Option<&'static str>] = &[
        check_first_run,
        check_run_25km,
        check_run_100km,
        check_run_250km,
        check_run_500km,
        check_run_1000km,
        check_run_5k_once,
        check_run_10k_once,
        check_run_half_once,
        check_run_marathon_once,
        check_pace_sub6,
        check_pace_sub5,
        check_pace_sub430,
        check_consistent_pace,
        check_runs_50,
        check_runs_100,
        check_runs_365,
        check_streak_3,
        check_streak_7,
        check_streak_14,
        check_streak_30,
        check_comeback,
        check_night_owl,
        check_early_bird,
        check_new_years_runner,
        check_speedy_upload,
        check_explorer,
        check_monday_10,
        check_pr_machine,
    ];

    checks
        .iter()
        .filter_map(|f| f(ctx))
        .filter(|slug| !ctx.already_unlocked.contains(*slug))
        .collect()
}

fn check_first_run(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.total_runs >= 1 { Some("first_run") } else { None }
}

fn check_run_25km(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.total_distance_m >= 25_000.0 { Some("run_25km") } else { None }
}

fn check_run_100km(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.total_distance_m >= 100_000.0 { Some("run_100km") } else { None }
}

fn check_run_250km(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.total_distance_m >= 250_000.0 { Some("run_250km") } else { None }
}

fn check_run_500km(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.total_distance_m >= 500_000.0 { Some("run_500km") } else { None }
}

fn check_run_1000km(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.total_distance_m >= 1_000_000.0 { Some("run_1000km") } else { None }
}

fn check_run_5k_once(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.activity_distance_m >= 5_000.0 { Some("run_5k_once") } else { None }
}

fn check_run_10k_once(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.activity_distance_m >= 10_000.0 { Some("run_10k_once") } else { None }
}

fn check_run_half_once(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.activity_distance_m >= 21_100.0 { Some("run_half_once") } else { None }
}

fn check_run_marathon_once(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.activity_distance_m >= 42_195.0 { Some("run_marathon_once") } else { None }
}

fn check_pace_sub6(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.activity_pace_min_per_km > 0.0 && ctx.activity_pace_min_per_km < 6.0 {
        Some("pace_sub6")
    } else {
        None
    }
}

fn check_pace_sub5(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.activity_pace_min_per_km > 0.0 && ctx.activity_pace_min_per_km < 5.0 {
        Some("pace_sub5")
    } else {
        None
    }
}

fn check_pace_sub430(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.activity_pace_min_per_km > 0.0 && ctx.activity_pace_min_per_km < 4.5 {
        Some("pace_sub430")
    } else {
        None
    }
}

/// 3 consecutive runs with pace within 0.167 min/km (10 sec/km) of each other.
fn check_consistent_pace(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.recent_paces.len() < 3 {
        return None;
    }
    let latest = &ctx.recent_paces[..3];
    let max = latest.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min = latest.iter().cloned().fold(f64::INFINITY, f64::min);
    if max - min <= 10.0 / 60.0 {
        Some("consistent_pace")
    } else {
        None
    }
}

fn check_runs_50(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.total_runs >= 50 { Some("runs_50") } else { None }
}

fn check_runs_100(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.total_runs >= 100 { Some("runs_100") } else { None }
}

fn check_runs_365(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.total_runs >= 365 { Some("runs_365") } else { None }
}

fn check_streak_3(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.current_streak >= 3 { Some("streak_3") } else { None }
}

fn check_streak_7(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.current_streak >= 7 { Some("streak_7") } else { None }
}

fn check_streak_14(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.current_streak >= 14 { Some("streak_14") } else { None }
}

fn check_streak_30(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.current_streak >= 30 { Some("streak_30") } else { None }
}

fn check_comeback(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.had_long_gap { Some("comeback") } else { None }
}

fn check_night_owl(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.activity_start.hour() >= 22 { Some("night_owl") } else { None }
}

fn check_early_bird(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.activity_start.hour() < 6 { Some("early_bird") } else { None }
}

fn check_new_years_runner(ctx: &CheckContext) -> Option<&'static str> {
    let d = ctx.activity_start.date_naive();
    if d.month() == 1 && d.day() == 1 { Some("new_years_runner") } else { None }
}

fn check_speedy_upload(ctx: &CheckContext) -> Option<&'static str> {
    // 3:30/km = 3.5 min/km
    if ctx.activity_pace_min_per_km > 0.0 && ctx.activity_pace_min_per_km < 3.5 {
        Some("speedy_upload")
    } else {
        None
    }
}

fn check_explorer(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.months_with_runs.len() >= 5 { Some("explorer") } else { None }
}

fn check_monday_10(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.monday_run_count >= 10 { Some("monday_10") } else { None }
}

fn check_pr_machine(ctx: &CheckContext) -> Option<&'static str> {
    if ctx.pr_count >= 3 { Some("personal_best_streak") } else { None }
}
