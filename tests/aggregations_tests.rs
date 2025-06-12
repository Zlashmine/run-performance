// #[cfg(test)]
// mod tests {
//     use activity_api::{activities::models::Activity, aggregate::aggretate_activities};
//     use chrono::NaiveDateTime;
//     use uuid::Uuid;

//     fn create_activity(date_str: &str, activity_type: &str, distance: f32, pace: f32) -> Activity {
//         Activity {
//             id: Uuid::new_v4(),
//             user_id: Uuid::new_v4(),
//             name: "Test".to_string(),
//             activity_type: activity_type.to_string(),
//             distance,
//             duration: "00:30:00".to_string(),
//             average_pace: pace,
//             average_speed: 10.0,
//             calories: 100.0,
//             climb: 50.0,
//             date: NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S").unwrap(),
//             gps_file: "test.gpx".to_string(),
//         }
//     }

//     #[test]
//     fn test_aggregates_per_type_and_month() {
//         let activities = vec![
//             create_activity("2024-01-05 08:00:00", "Running", 5.0, 5.0),
//             create_activity("2024-01-10 08:00:00", "Running", 10.0, 5.5),
//             create_activity("2024-02-15 08:00:00", "Running", 7.0, 6.0),
//             create_activity("2024-01-20 08:00:00", "Cycling", 20.0, 3.0),
//         ];

//         let (agg, time_agg) = aggretate_activities(&activities);

//         // Top-level
//         assert_eq!(agg.len(), 2);
//         assert_eq!(agg["Running"].0.total_activities, 3);
//         assert_eq!(agg["Running"].0.total_distance, 22.0);

//         // Monthly
//         let run_months = time_agg.get("Running").unwrap();
//         assert_eq!(run_months["2024-01"].total_activities, 2);
//         assert_eq!(run_months["2024-02"].total_activities, 1);

//         let cyc_months = time_agg.get("Cycling").unwrap();
//         assert_eq!(cyc_months["2024-01"].total_distance, 20.0);
//     }

//     #[test]
//     fn test_empty_input() {
//         let (agg, time_agg) = aggretate_activities(&vec![]);
//         assert!(agg.is_empty());
//         assert!(time_agg.is_empty());
//     }

//     #[test]
//     fn test_time_aggregation_sums_match_totals() {
//         let activities = vec![
//             create_activity("2024-01-05 08:00:00", "Running", 5.0, 5.0),
//             create_activity("2024-01-10 08:00:00", "Running", 10.0, 5.5),
//             create_activity("2024-02-15 08:00:00", "Running", 7.0, 6.0),
//             create_activity("2024-03-01 08:00:00", "Running", 3.0, 4.0),
//             create_activity("2024-01-20 08:00:00", "Cycling", 20.0, 3.0),
//             create_activity("2024-02-25 08:00:00", "Cycling", 10.0, 3.5),
//         ];

//         let (total, time_agg) = aggretate_activities(&activities);

//         for (activity_type, total_agg) in &total {
//             let monthly = time_agg
//                 .get(activity_type)
//                 .expect("Missing time aggregation");

//             let mut total_distance = 0.0;
//             let mut total_activities = 0;

//             for agg in monthly.values() {
//                 total_distance += agg.total_distance;
//                 total_activities += agg.total_activities;
//             }

//             assert!(
//                 (total_distance - total_agg.0.total_distance).abs() < f32::EPSILON,
//                 "Distance mismatch for {}",
//                 activity_type
//             );

//             assert_eq!(
//                 total_activities, total_agg.0.total_activities,
//                 "Activity count mismatch for {}",
//                 activity_type
//             );
//         }
//     }

//     #[test]
//     fn test_advanced_aggregation_fields() {
//         let activities = vec![
//             create_activity("2024-01-01 06:00:00", "Running", 5.0, 5.0), // Tuesday
//             create_activity("2024-01-02 07:00:00", "Running", 6.0, 5.5), // Wednesday
//             create_activity("2024-01-03 08:00:00", "Running", 7.0, 6.0), // Thursday
//             create_activity("2024-01-04 09:00:00", "Running", 8.0, 4.8), // Friday
//             create_activity("2024-01-05 10:00:00", "Running", 9.0, 4.9), // Saturday
//             create_activity("2024-01-06 11:00:00", "Running", 10.0, 4.7), // Sunday
//             create_activity("2024-01-07 12:00:00", "Running", 11.0, 4.6), // Monday
//         ];

//         let (agg, _) = aggretate_activities(&activities);
//         let (_basic, advanced) = agg.get("Running").expect("Expected running aggregation");

//         assert_eq!(advanced.longest_streak_days, 7);
//         assert_eq!(advanced.top_3_fastest_weekdays.len(), 3);
//         assert!(advanced.max_daily_calories > 0.0);
//         assert!(advanced.top_speeds.len() <= 3);
//         assert!(advanced.max_climb > 0.0);
//         assert!(advanced.most_frequent_weekday.is_some());
//         assert!(advanced.most_consistent_week.is_some());

//         // Fun stats
//         assert!(
//             advanced.slowest_pace > 0.0,
//             "Should have a slowest pace value"
//         );
//         assert!(
//             advanced.speed_demon_hour.is_some(),
//             "Should detect the speed demon hour"
//         );
//         assert!(
//             advanced.sweatiest_week.is_some(),
//             "Should determine the sweatiest week"
//         );
//         assert!(
//             advanced.most_skipped_weekday.is_some(),
//             "Should find the skipped weekday"
//         );
//         assert!(advanced.weekend_ratio > 0.0, "Weekend ratio should be > 0");
//         assert!(
//             advanced.pace_std_dev > 0.0,
//             "Standard deviation of pace should be > 0"
//         );
//         assert!(
//             advanced.max_effort_cal_per_min > 0.0,
//             "Max effort per minute should be > 0"
//         );
//     }

//     #[test]
//     fn test_advanced_aggregation_edge_cases() {
//         let empty = vec![];
//         // Empty input
//         let (agg, _) = aggretate_activities(&empty);
//         assert!(agg.is_empty());

//         // One entry
//         let activities = vec![create_activity("2024-01-01 06:00:00", "Running", 10.0, 6.0)];

//         let (agg, _) = aggretate_activities(&activities);
//         let (_basic, advanced) = agg.get("Running").unwrap();

//         assert_eq!(advanced.longest_streak_days, 0);
//         assert_eq!(advanced.top_3_fastest_weekdays.len(), 1);
//         assert_eq!(advanced.max_daily_calories, 100.0);
//         assert_eq!(advanced.top_speeds.len(), 1);
//         assert_eq!(advanced.max_climb, 50.0);
//         assert!(advanced.most_frequent_weekday.is_some());
//         assert!(advanced.most_consistent_week.is_some());

//         // Fun stats still calculable with 1 input
//         assert_eq!(advanced.slowest_pace, 6.0);
//         assert!(advanced.speed_demon_hour.is_some());
//         assert!(advanced.sweatiest_week.is_some());
//         assert!(advanced.most_skipped_weekday.is_some());
//         assert!(advanced.weekend_ratio >= 0.0);
//         assert_eq!(advanced.pace_std_dev, 0.0); // only one = no deviation
//         assert!(advanced.max_effort_cal_per_min > 0.0);
//     }
// }
