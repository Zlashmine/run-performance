use crate::aggregate::models::{ScoringConfig, ScoringRule};
use std::collections::HashMap;

use super::models::{ActivitiesAggregation, AdvancedAggregation};

pub fn default_scoring_config() -> ScoringConfig {
    ScoringConfig {
        rules: HashMap::from([
            (
                "average_pace".to_string(),
                ScoringRule {
                    base: 100,
                    multiplier: 250.0,
                },
            ),
            (
                "best_pace".to_string(),
                ScoringRule {
                    base: 100,
                    multiplier: 300.0,
                },
            ),
            (
                "total_distance".to_string(),
                ScoringRule {
                    base: 0,
                    multiplier: 0.5,
                },
            ),
            (
                "average_distance".to_string(),
                ScoringRule {
                    base: 0,
                    multiplier: 50.0,
                },
            ),
            (
                "best_distance".to_string(),
                ScoringRule {
                    base: 100,
                    multiplier: 25.0,
                },
            ),
            (
                "max_climb".to_string(),
                ScoringRule {
                    base: 0,
                    multiplier: 1.0,
                },
            ),
            (
                "longest_streak_days".to_string(),
                ScoringRule {
                    base: 0,
                    multiplier: 100.0,
                },
            ),
            (
                "longest_streak_weeks".to_string(),
                ScoringRule {
                    base: 0,
                    multiplier: 50.0,
                },
            ),
            (
                "current_weekly_streak".to_string(),
                ScoringRule {
                    base: 0,
                    multiplier: 50.0,
                },
            ),
            (
                "max_effort_cal_per_min".to_string(),
                ScoringRule {
                    base: 0,
                    multiplier: 20.0,
                },
            ),
            (
                "pace_std_dev".to_string(),
                ScoringRule {
                    base: 0,
                    multiplier: 200.0,
                },
            ),
            (
                "max_daily_calories".to_string(),
                ScoringRule {
                    base: 0,
                    multiplier: 0.25, // 4000 * 0.25 = 1000 points
                },
            ),
            (
                "total_activities".to_string(),
                ScoringRule {
                    base: 0,
                    multiplier: 5.0, // each activity = 5 points
                },
            ),
        ]),
    }
}

pub fn calculate_scores(
    basic: &ActivitiesAggregation,
    advanced: &Option<AdvancedAggregation>,
    config: &ScoringConfig,
) -> HashMap<String, i32> {
    let mut scores = HashMap::new();

    for (key, rule) in &config.rules {
        let score = match key.as_str() {
            "average_pace" => {
                let pace_diff = 6.0 - basic.average_pace;
                rule.base as f32 + (pace_diff * rule.multiplier)
            }
            "total_distance" => basic.total_distance * rule.multiplier + rule.base as f32,
            "average_distance" => basic.average_distance * rule.multiplier + rule.base as f32,
            "best_distance" => basic.best_distance * rule.multiplier + rule.base as f32,
            "best_pace" => {
                let pace_diff = 6.0 - basic.best_pace;
                rule.base as f32 + (pace_diff * rule.multiplier)
            }

            "max_climb" => advanced
                .as_ref()
                .map_or(0.0, |a| a.max_climb * rule.multiplier + rule.base as f32),
            "longest_streak_days" => advanced.as_ref().map_or(0.0, |a| {
                a.longest_streak_days as f32 * rule.multiplier + rule.base as f32
            }),
            "longest_streak_weeks" => advanced.as_ref().map_or(0.0, |a| {
                a.longest_streak_weeks as f32 * rule.multiplier + rule.base as f32
            }),
            "current_weekly_streak" => advanced.as_ref().map_or(0.0, |a| {
                a.current_weekly_streak as f32 * rule.multiplier + rule.base as f32
            }),
            "max_effort_cal_per_min" => advanced.as_ref().map_or(0.0, |a| {
                a.max_effort_cal_per_min * rule.multiplier + rule.base as f32
            }),
            "pace_std_dev" => advanced.as_ref().map_or(0.0, |a| {
                let inverse_dev = (3.0 - a.pace_std_dev).max(0.0);
                rule.base as f32 + (inverse_dev * rule.multiplier)
            }),
            "max_daily_calories" => advanced.as_ref().map_or(0.0, |a| {
                a.max_daily_calories * rule.multiplier + rule.base as f32
            }),
            "total_activities" => {
                basic.total_activities as f32 * rule.multiplier + rule.base as f32
            }
            _ => 0.0,
        };

        let score_capped = score.clamp(0.0, 1000.0) as i32;
        scores.insert(key.clone(), score_capped);
    }

    scores
}

pub fn classify_score(total: i32) -> String {
    match total {
        0..=99 => "Unranked".to_string(),
        100..=299 => "Bronze".to_string(),
        300..=499 => "Silver".to_string(),
        500..=699 => "Gold".to_string(),
        700..=999 => "Platinum".to_string(),
        _ => "Titanium".to_string(),
    }
}
