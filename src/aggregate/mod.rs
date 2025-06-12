use crate::activities::models::Activity;
use chrono::{NaiveTime, Timelike};
use models::{
    ActivitiesAggregation, AdvancedAggregation, AggregationDTO, ScoreDetail, ScoreSummary,
    ScoringConfig,
};
use std::collections::HashMap;
use utils::{calculate_scores, classify_score};

pub mod models;
mod utils;

pub fn aggretate_activities(
    activities: &Vec<Activity>,
) -> (
    HashMap<String, AggregationDTO>,
    HashMap<String, HashMap<String, ActivitiesAggregation>>,
) {
    let mut activity_types: HashMap<String, Vec<Activity>> = HashMap::new();
    let mut time_groups: HashMap<String, HashMap<String, Vec<Activity>>> = HashMap::new();

    for activity in activities {
        let activity_type = activity.activity_type.clone();

        activity_types
            .entry(activity_type.clone())
            .or_default()
            .push(activity.clone());

        let month_key = activity.date.format("%Y-%m").to_string();

        time_groups
            .entry(activity_type.clone())
            .or_default()
            .entry(month_key)
            .or_default()
            .push(activity.clone());
    }

    let config = utils::default_scoring_config();

    let mut aggregation_map = HashMap::new();
    for (activity_type, acts) in &activity_types {
        let basic = aggregate_activities(acts);
        let advanced = compute_advanced_aggregation(acts);

        let score_summary = calculate_score_summary(&basic, &Some(advanced.clone()), &config);

        aggregation_map.insert(
            activity_type.clone(),
            AggregationDTO {
                basic,
                advanced: Some(advanced),
                scores: score_summary,
            },
        );
    }

    let mut time_aggregations = HashMap::new();
    for (activity_type, month_map) in time_groups {
        let mut inner = HashMap::new();
        for (month, acts) in month_map {
            inner.insert(month, aggregate_activities(&acts));
        }
        time_aggregations.insert(activity_type, inner);
    }

    (aggregation_map, time_aggregations)
}

fn aggregate_activities(activities: &[Activity]) -> ActivitiesAggregation {
    let total_activities = activities.len() as u32;
    let total_distance = activities.iter().map(|a| a.distance).sum();

    let total_seconds: f32 = activities
        .iter()
        .map(|a| a.average_pace * a.distance * 60.0)
        .sum();

    let average_pace = if total_distance > 0.0 {
        let pace_seconds_per_km: f32 = total_seconds / total_distance;
        let minutes = ((pace_seconds_per_km) / (60.0_f32)).floor();
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

    let best_distance = activities.iter().map(|a| a.distance).fold(0.0, f32::max);

    let best_pace = activities
        .iter()
        .map(|a| a.average_pace)
        .fold(f32::INFINITY, f32::min);

    ActivitiesAggregation {
        total_activities,
        total_distance,
        average_pace,
        average_distance,
        best_distance,
        best_pace: if best_pace.is_infinite() {
            0.0
        } else {
            f32::min(best_pace, average_pace)
        },
    }
}

fn compute_advanced_aggregation(activities: &[Activity]) -> AdvancedAggregation {
    use chrono::{Datelike, IsoWeek, NaiveDate};
    use std::collections::HashMap;

    let mut dates: Vec<_> = activities.iter().map(|a| a.date.date()).collect();
    dates.sort();
    dates.dedup();

    let mut longest_streak = 0;
    let mut current_streak = 1;

    for w in dates.windows(2) {
        if (w[1] - w[0]).num_days() == 1 {
            current_streak += 1;
            longest_streak = longest_streak.max(current_streak);
        } else {
            current_streak = 1;
        }
    }

    let mut weeks: Vec<_> = activities.iter().map(|a| a.date.iso_week()).collect();
    weeks.sort();
    weeks.dedup();

    let mut longest_week_streak = 0;
    let mut current_week_streak = 1;

    for w in weeks.windows(2) {
        if w[0].year() == w[1].year() && w[0].week() + 1 == w[1].week()
            || w[0].year() + 1 == w[1].year() && w[0].week() == 52 && w[1].week() == 1
        {
            current_week_streak += 1;
            longest_week_streak = longest_week_streak.max(current_week_streak);
        } else {
            current_week_streak = 1;
        }
    }

    let today = chrono::Utc::now().naive_utc().date();
    let current_week = today.iso_week();

    let mut current_weekly_streak = 0;
    let week_set: std::collections::HashSet<chrono::IsoWeek> = weeks.into_iter().collect();

    let mut week_iter = current_week;
    loop {
        if week_set.contains(&week_iter) {
            current_weekly_streak += 1;

            // move to previous week
            let prev_date = chrono::NaiveDate::from_isoywd_opt(
                week_iter.year(),
                week_iter.week(),
                chrono::Weekday::Mon,
            )
            .unwrap()
            .pred_opt()
            .unwrap();
            week_iter = prev_date.iso_week();
        } else {
            break;
        }
    }

    let mut weekday_totals: HashMap<String, (f32, u32)> = HashMap::new();
    let mut weekday_counts: HashMap<String, u32> = HashMap::new();
    let mut week_pace_map: HashMap<IsoWeek, Vec<f32>> = HashMap::new();
    let mut day_calories: HashMap<NaiveDate, f32> = HashMap::new();
    let mut speed_list: Vec<f32> = vec![];
    let mut max_climb = 0.0;

    for a in activities {
        let weekday = a.date.weekday().to_string();
        *weekday_counts.entry(weekday.clone()).or_default() += 1;

        let entry = weekday_totals.entry(weekday).or_default();
        entry.0 += a.average_pace;
        entry.1 += 1;

        week_pace_map
            .entry(a.date.iso_week())
            .or_default()
            .push(a.average_pace);
        *day_calories.entry(a.date.date()).or_default() += a.calories;

        speed_list.push(a.average_speed);
        let max = f32::max(max_climb, a.climb); // max_climb.max(a.climb as f32);

        max_climb = max;
    }

    let mut pace_by_day: Vec<(String, f32)> = weekday_totals
        .into_iter()
        .map(|(day, (total, count))| (day, total / count as f32))
        .collect();
    pace_by_day.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    pace_by_day.truncate(3);

    let mut week_variances: Vec<(IsoWeek, f32)> = week_pace_map
        .iter()
        .map(|(week, paces)| {
            let mean = paces.iter().copied().sum::<f32>() / paces.len() as f32;
            let var = paces.iter().map(|p| (p - mean).powi(2)).sum::<f32>() / paces.len() as f32;
            (*week, var)
        })
        .collect();
    week_variances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    let most_consistent_week = week_variances
        .first()
        .map(|w| format!("{}-W{}", w.0.year(), w.0.week()));

    let max_daily_calories = day_calories.values().copied().fold(0.0, f32::max);

    speed_list.sort_by(|a, b| b.partial_cmp(a).unwrap());
    let top_speeds = speed_list.into_iter().take(3).collect();

    let most_frequent_weekday = weekday_counts
        .clone()
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(day, _)| day);

    let mut hour_buckets: HashMap<u32, (f32, u32)> = HashMap::new();
    let mut calories_per_week: HashMap<IsoWeek, f32> = HashMap::new();
    // let mut weekday_counter: HashMap<String, u32> = HashMap::new();
    let mut pace_list = vec![];
    let mut max_effort_cal_per_min = 0.0;
    let mut slowest_pace = 0.0;
    let mut total_weekend_sessions = 0;
    let mut total_sessions = 0;

    for a in activities {
        let weekday = a.date.weekday().to_string();
        *weekday_counts.entry(weekday.clone()).or_default() += 1;

        // Speed Demon Hour
        let hour = a.date.hour();
        let h_entry = hour_buckets.entry(hour).or_default();
        h_entry.0 += a.average_pace;
        h_entry.1 += 1;

        // Sweatiest Week
        *calories_per_week.entry(a.date.iso_week()).or_default() += a.calories;

        // Pace Roulette & Sloth Mode
        pace_list.push(a.average_pace);
        slowest_pace = f32::max(slowest_pace, a.average_pace);

        // Max Effort Session
        if let Ok(time) = NaiveTime::parse_from_str(&a.duration, "%H:%M:%S") {
            let duration_mins = time.num_seconds_from_midnight() as f32 / 60.0;
            if duration_mins > 0.0 {
                let cal_per_min = a.calories / duration_mins;
                max_effort_cal_per_min = f32::max(max_effort_cal_per_min, cal_per_min);
            }
        }

        // Weekend Warrior
        if matches!(
            a.date.weekday(),
            chrono::Weekday::Sat | chrono::Weekday::Sun
        ) {
            total_weekend_sessions += 1;
        }

        total_sessions += 1;
    }

    // Speed Demon Hour (lowest average pace by hour)
    let speed_demon_hour = hour_buckets
        .into_iter()
        .map(|(h, (total, count))| (h, total / count as f32))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(h, _)| format!("{:02}:00", h));

    // Sweatiest Week
    let sweatiest_week = calories_per_week
        .into_iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(w, _)| format!("{}-W{}", w.year(), w.week()));

    // Most Skipped Day
    let most_skipped_weekday = weekday_counts
        .clone()
        .into_iter()
        .min_by_key(|(_, count)| *count)
        .map(|(day, _)| day);

    // Pace Standard Deviation
    let pace_std_dev = {
        let mean = pace_list.iter().sum::<f32>() / pace_list.len() as f32;
        (pace_list.iter().map(|p| (p - mean).powi(2)).sum::<f32>() / pace_list.len() as f32).sqrt()
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
        total_score += i32::min(score, 1000);

        breakdown.insert(key, ScoreDetail { score, level });
    }

    let level = classify_score(total_score);

    ScoreSummary {
        total_score,
        level,
        breakdown,
    }
}
