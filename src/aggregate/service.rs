/// Pure aggregation functions — no DB, no HTTP, no global state.
///
/// All public functions take slices of Activity and return aggregation structs.
/// Safe to call from any context (handler, test, background job).
use crate::activities::models::Activity;
use chrono::{Datelike, IsoWeek, NaiveDate, NaiveTime, Timelike};
use std::collections::HashMap;

use super::{
    models::{
        ActivitiesAggregation, AdvancedAggregation, AggregationDTO, ScoreDetail, ScoreSummary,
        ScoringConfig,
    },
    scoring::{calculate_scores, classify_score, default_scoring_config},
};

/// Top-level entry point: splits activities by type then aggregates each group.
///
/// Returns:
/// - A map of activity_type → AggregationDTO (basic + advanced stats + scores)
/// - A map of activity_type → month_key → ActivitiesAggregation (for time-series charts)
pub fn aggregate_activities(
    activities: &[Activity],
) -> (
    HashMap<String, AggregationDTO>,
    HashMap<String, HashMap<String, ActivitiesAggregation>>,
) {
    let mut activity_types: HashMap<String, Vec<Activity>> = HashMap::new();
    let mut time_groups: HashMap<String, HashMap<String, Vec<Activity>>> = HashMap::new();

    for activity in activities {
        let activity_type = activity.activity_type.clone();
        let month_key = activity.date.format("%Y-%m").to_string();

        activity_types
            .entry(activity_type.clone())
            .or_default()
            .push(activity.clone());

        time_groups
            .entry(activity_type.clone())
            .or_default()
            .entry(month_key)
            .or_default()
            .push(activity.clone());
    }

    let config = default_scoring_config();

    let mut aggregation_map = HashMap::new();
    for (activity_type, acts) in &activity_types {
        let basic = compute_basic_aggregation(acts);
        let advanced = compute_advanced_aggregation(acts);
        let scores = calculate_score_summary(&basic, &Some(advanced.clone()), &config);

        aggregation_map.insert(
            activity_type.clone(),
            AggregationDTO {
                basic,
                advanced: Some(advanced),
                scores,
            },
        );
    }

    let mut time_aggregations = HashMap::new();
    for (activity_type, month_map) in time_groups {
        let mut inner = HashMap::new();
        for (month, acts) in month_map {
            inner.insert(month, compute_basic_aggregation(&acts));
        }
        time_aggregations.insert(activity_type, inner);
    }

    (aggregation_map, time_aggregations)
}

pub fn compute_basic_aggregation(activities: &[Activity]) -> ActivitiesAggregation {
    let total_activities = activities.len() as u32;
    let total_distance: f32 = activities.iter().map(|a| a.distance).sum();

    let total_seconds: f32 = activities
        .iter()
        .map(|a| a.average_pace * a.distance * 60.0)
        .sum();

    let average_pace = if total_distance > 0.0 {
        let pace_seconds_per_km = total_seconds / total_distance;
        let minutes = (pace_seconds_per_km / 60.0).floor();
        let seconds = (pace_seconds_per_km % 60.0) / 100.0;
        minutes + seconds
    } else {
        0.0
    };

    let average_distance = if total_activities > 0 {
        total_distance / total_activities as f32
    } else {
        0.0
    };

    let best_distance = activities
        .iter()
        .map(|a| a.distance)
        .fold(0.0_f32, f32::max);

    // Lower pace = faster. best_pace is the minimum pace value observed.
    // Q8 fix: do NOT cap best_pace at average_pace — best_pace can legitimately be better.
    let raw_best_pace = activities
        .iter()
        .map(|a| a.average_pace)
        .fold(f32::INFINITY, f32::min);

    let best_pace = if raw_best_pace.is_infinite() {
        0.0
    } else {
        raw_best_pace
    };

    ActivitiesAggregation {
        total_activities,
        total_distance,
        average_pace,
        average_distance,
        best_distance,
        best_pace,
    }
}

/// Computes advanced statistics in a **single pass** over activities (Q5 fix).
///
/// All metrics — streaks, weekday patterns, calories, effort, etc. — are derived
/// in one iteration. The only secondary pass is deduplication of sorted date/week
/// vectors which is O(n log n) and kept separate for clarity.
pub fn compute_advanced_aggregation(activities: &[Activity]) -> AdvancedAggregation {
    if activities.is_empty() {
        return AdvancedAggregation::default();
    }

    // ── Single-pass collection ─────────────────────────────────────────────
    let mut weekday_pace_acc: HashMap<String, (f32, u32)> = HashMap::new();
    let mut weekday_counts: HashMap<String, u32> = HashMap::new();
    let mut week_pace_map: HashMap<IsoWeek, Vec<f32>> = HashMap::new();
    let mut day_calories: HashMap<NaiveDate, f32> = HashMap::new();
    let mut calories_per_week: HashMap<IsoWeek, f32> = HashMap::new();
    let mut hour_buckets: HashMap<u32, (f32, u32)> = HashMap::new();
    let mut pace_list: Vec<f32> = Vec::new();
    let mut speed_list: Vec<f32> = Vec::new();
    let mut max_climb: f32 = 0.0;
    let mut max_effort_cal_per_min: f32 = 0.0;
    let mut slowest_pace: f32 = 0.0;
    let mut total_weekend_sessions: u32 = 0;
    let total_sessions = activities.len() as u32;

    // Collect dates and weeks for streak computation (needs sort before use)
    let mut raw_dates: Vec<NaiveDate> = Vec::new();
    let mut raw_weeks: Vec<IsoWeek> = Vec::new();

    for a in activities {
        let weekday = a.date.weekday().to_string();

        *weekday_counts.entry(weekday.clone()).or_default() += 1;

        let pace_acc = weekday_pace_acc.entry(weekday.clone()).or_default();
        pace_acc.0 += a.average_pace;
        pace_acc.1 += 1;

        let week = a.date.iso_week();
        week_pace_map.entry(week).or_default().push(a.average_pace);
        *day_calories.entry(a.date.date()).or_default() += a.calories;
        *calories_per_week.entry(week).or_default() += a.calories;

        let h_entry = hour_buckets.entry(a.date.hour()).or_default();
        h_entry.0 += a.average_pace;
        h_entry.1 += 1;

        pace_list.push(a.average_pace);
        speed_list.push(a.average_speed);
        slowest_pace = f32::max(slowest_pace, a.average_pace);
        max_climb = f32::max(max_climb, a.climb);

        if let Ok(time) = NaiveTime::parse_from_str(&a.duration, "%H:%M:%S") {
            let duration_mins = time.num_seconds_from_midnight() as f32 / 60.0;
            if duration_mins > 0.0 {
                let cal_per_min = a.calories / duration_mins;
                max_effort_cal_per_min = f32::max(max_effort_cal_per_min, cal_per_min);
            }
        }

        if matches!(
            a.date.weekday(),
            chrono::Weekday::Sat | chrono::Weekday::Sun
        ) {
            total_weekend_sessions += 1;
        }

        raw_dates.push(a.date.date());
        raw_weeks.push(week);
    }

    // ── Streak computation (requires sorted, deduped sequences) ───────────
    raw_dates.sort();
    raw_dates.dedup();

    let mut longest_streak: u32 = if raw_dates.is_empty() { 0 } else { 1 };
    let mut current_streak: u32 = 1;
    for w in raw_dates.windows(2) {
        if (w[1] - w[0]).num_days() == 1 {
            current_streak += 1;
            longest_streak = longest_streak.max(current_streak);
        } else {
            current_streak = 1;
        }
    }

    raw_weeks.sort();
    raw_weeks.dedup();

    let mut longest_week_streak: u32 = if raw_weeks.is_empty() { 0 } else { 1 };
    let mut current_week_streak: u32 = 1;
    for w in raw_weeks.windows(2) {
        let consecutive = (w[0].year() == w[1].year() && w[0].week() + 1 == w[1].week())
            || (w[0].year() + 1 == w[1].year() && w[0].week() == 52 && w[1].week() == 1);
        if consecutive {
            current_week_streak += 1;
            longest_week_streak = longest_week_streak.max(current_week_streak);
        } else {
            current_week_streak = 1;
        }
    }

    let today = chrono::Utc::now().naive_utc().date();
    let week_set: std::collections::HashSet<IsoWeek> = raw_weeks.into_iter().collect();
    let mut current_weekly_streak: u32 = 0;
    let mut week_iter = today.iso_week();
    loop {
        if week_set.contains(&week_iter) {
            current_weekly_streak += 1;
            let prev_date = chrono::NaiveDate::from_isoywd_opt(
                week_iter.year(),
                week_iter.week(),
                chrono::Weekday::Mon,
            )
            .and_then(|d| d.pred_opt())
            .unwrap_or(today);
            week_iter = prev_date.iso_week();
        } else {
            break;
        }
    }

    // ── Derived metrics ────────────────────────────────────────────────────
    let mut pace_by_day: Vec<(String, f32)> = weekday_pace_acc
        .into_iter()
        .map(|(day, (total, count))| (day, total / count as f32))
        .collect();
    pace_by_day.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    pace_by_day.truncate(3);

    let mut week_variances: Vec<(IsoWeek, f32)> = week_pace_map
        .iter()
        .map(|(week, paces)| {
            let mean = paces.iter().copied().sum::<f32>() / paces.len() as f32;
            let var = paces.iter().map(|p| (p - mean).powi(2)).sum::<f32>() / paces.len() as f32;
            (*week, var)
        })
        .collect();
    week_variances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let most_consistent_week = week_variances
        .first()
        .map(|w| format!("{}-W{}", w.0.year(), w.0.week()));

    let max_daily_calories = day_calories.values().copied().fold(0.0_f32, f32::max);

    speed_list.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let top_speeds: Vec<f32> = speed_list.into_iter().take(3).collect();

    let most_frequent_weekday = weekday_counts
        .iter()
        .max_by_key(|(_, &count)| count)
        .map(|(day, _)| day.clone());

    let most_skipped_weekday = weekday_counts
        .iter()
        .min_by_key(|(_, &count)| count)
        .map(|(day, _)| day.clone());

    let speed_demon_hour = hour_buckets
        .into_iter()
        .map(|(h, (total, count))| (h, total / count as f32))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(h, _)| format!("{:02}:00", h));

    let sweatiest_week = calories_per_week
        .into_iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(w, _)| format!("{}-W{}", w.year(), w.week()));

    let pace_std_dev = if pace_list.len() > 1 {
        let mean = pace_list.iter().sum::<f32>() / pace_list.len() as f32;
        (pace_list.iter().map(|p| (p - mean).powi(2)).sum::<f32>() / pace_list.len() as f32).sqrt()
    } else {
        0.0
    };

    let weekend_ratio = if total_sessions > 0 {
        total_weekend_sessions as f32 / total_sessions as f32
    } else {
        0.0
    };

    AdvancedAggregation {
        longest_streak_days: longest_streak,
        longest_streak_weeks: longest_week_streak,
        current_weekly_streak,
        top_3_fastest_weekdays: pace_by_day,
        most_consistent_week,
        max_daily_calories,
        top_speeds,
        max_climb,
        most_frequent_weekday,
        slowest_pace,
        speed_demon_hour,
        sweatiest_week,
        most_skipped_weekday,
        weekend_ratio,
        pace_std_dev,
        max_effort_cal_per_min,
    }
}

pub fn calculate_score_summary(
    basic: &ActivitiesAggregation,
    advanced: &Option<AdvancedAggregation>,
    config: &ScoringConfig,
) -> ScoreSummary {
    let raw_scores = calculate_scores(basic, advanced, config);

    let mut breakdown = HashMap::new();
    let mut total_score = 0;

    for (key, score) in raw_scores {
        let level = classify_score(score);
        total_score += score.min(1000);
        breakdown.insert(key, ScoreDetail { score, level });
    }

    ScoreSummary {
        total_score,
        level: classify_score(total_score),
        breakdown,
    }
}
