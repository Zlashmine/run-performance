use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

    // New fun fields:
    pub slowest_pace: f32,
    pub speed_demon_hour: Option<String>,
    pub sweatiest_week: Option<String>,
    pub most_skipped_weekday: Option<String>,
    pub weekend_ratio: f32,
    pub pace_std_dev: f32,
    pub max_effort_cal_per_min: f32,
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
