use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::Datelike;

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ActivitiesAggregation {
    pub total_activities: u32,
    pub total_distance: f32,
    pub average_pace: f32,
    pub average_distance: f32,
    pub best_distance: f32,
    pub best_pace: f32,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema, Clone)]
pub struct AdvancedAggregation {
    pub longest_streak_days: u32,
    pub longest_streak_weeks: u32,
    pub current_weekly_streak: u32,
    pub top_3_fastest_weekdays: Vec<(String, f32)>,
    pub most_consistent_week: Option<String>,
    pub max_daily_calories: f32,
    pub top_speeds: Vec<f32>,
    pub max_climb: f32,
    pub most_frequent_weekday: Option<String>,
    pub slowest_pace: f32,
    pub speed_demon_hour: Option<String>,
    pub sweatiest_week: Option<String>,
    pub most_skipped_weekday: Option<String>,
    pub weekend_ratio: f32,
    pub pace_std_dev: f32,
    pub max_effort_cal_per_min: f32,
    // ── Streak detail ────────────────────────────────────────────────────
    /// Whether the user has at least one activity in the current ISO week.
    pub ran_this_week: bool,
    /// ISO weekdays remaining in the current week (Sun=0, Mon=6, …, Sat=1).
    pub days_until_week_end: u32,
    /// True when the streak is active but no run has been logged and
    /// ≤3 days remain in the current ISO week.
    pub streak_at_risk: bool,
    /// Number of activities logged in the current ISO week.
    pub streak_runs_this_week: u32,
    /// Total distance (km) logged in the current ISO week.
    pub streak_distance_this_week: f32,
    /// Total distance (km) across all weeks in the current streak window.
    pub streak_total_km: f32,
    /// Total activity count across all weeks in the current streak window.
    pub streak_total_runs: u32,
}

impl Default for AdvancedAggregation {
    fn default() -> Self {
        let today = chrono::Utc::now().naive_utc().date();
        let days_until_week_end = 6u32.saturating_sub(today.weekday().num_days_from_monday());
        AdvancedAggregation {
            longest_streak_days: 0,
            longest_streak_weeks: 0,
            current_weekly_streak: 0,
            top_3_fastest_weekdays: vec![],
            most_consistent_week: None,
            max_daily_calories: 0.0,
            top_speeds: vec![],
            max_climb: 0.0,
            most_frequent_weekday: None,
            slowest_pace: 0.0,
            speed_demon_hour: None,
            sweatiest_week: None,
            most_skipped_weekday: None,
            weekend_ratio: 0.0,
            pace_std_dev: 0.0,
            max_effort_cal_per_min: 0.0,
            ran_this_week: false,
            days_until_week_end,
            streak_at_risk: false,
            streak_runs_this_week: 0,
            streak_distance_this_week: 0.0,
            streak_total_km: 0.0,
            streak_total_runs: 0,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AggregationDTO {
    pub basic: ActivitiesAggregation,
    pub advanced: Option<AdvancedAggregation>,
    pub scores: ScoreSummary,
}

#[derive(Debug)]
pub struct ScoringRule {
    pub base: i32,
    pub multiplier: f32,
}

#[derive(Debug)]
pub struct ScoringConfig {
    pub rules: HashMap<String, ScoringRule>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ScoreSummary {
    pub total_score: i32,
    pub level: String,
    pub breakdown: HashMap<String, ScoreDetail>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ScoreDetail {
    pub score: i32,
    pub level: String,
}
