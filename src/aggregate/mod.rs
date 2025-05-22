use std::collections::HashMap;

use models::ActivitiesAggregation;

use crate::activities::models::Activity;

pub mod models;

pub fn aggretate_activities(activities: &Vec<Activity>) -> HashMap<String, ActivitiesAggregation> {
    let mut activity_types: HashMap<String, Vec<Activity>> = HashMap::new();

    for activity in activities {
        let activity_type = activity.activity_type.clone();

        activity_types
            .entry(activity_type)
            .or_default()
            .push(activity.clone());
    }

    let mut aggregation_map: HashMap<String, ActivitiesAggregation> = HashMap::new();

    for (activity_type, activities_for_type) in activity_types {
        aggregation_map.insert(activity_type, aggregate_activities(&activities_for_type));
    }

    aggregation_map
}

fn aggregate_activities(activities: &[Activity]) -> ActivitiesAggregation {
    let total_activities = activities.len() as u32;
    let total_distance = activities.iter().map(|a| a.distance).sum();

    let average_pace = if total_activities > 0 {
        activities.iter().map(|a| a.average_pace).sum::<f32>() / total_activities as f32
    } else {
        0.0
    };

    ActivitiesAggregation {
        total_activities,
        total_distance,
        average_pace,
    }
}
