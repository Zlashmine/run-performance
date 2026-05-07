/// Runkeeper adapter — wraps the existing CSV + GPX parser.
///
/// This adapter is used as the implementation of `ActivitySource` for the
/// Runkeeper file-upload flow. It converts an already-parsed set of
/// `Activity` + `TrackPoint` values into the canonical `NormalizedActivity`
/// representation so the ingestion pipeline remains source-agnostic.
use std::collections::HashMap;

use uuid::Uuid;

use crate::activities::models::{Activity, TrackPoint};

use super::normalized::{NormalizedActivity, NormalizedTrackPoint};

/// Convert a parsed `Activity` (from `activities::parser`) plus its
/// associated track points into a `NormalizedActivity`.
#[allow(dead_code)]
pub fn from_parsed(activity: Activity, track_points: Vec<TrackPoint>) -> NormalizedActivity {
    let normalized_tps = track_points
        .into_iter()
        .map(|tp| NormalizedTrackPoint {
            latitude: tp.latitude,
            longitude: tp.longitude,
            elevation: tp.elevation,
            time: tp.time,
            speed: tp.speed,
        })
        .collect();

    NormalizedActivity {
        source: "runkeeper".to_string(),
        external_id: None, // Runkeeper rows dedup by (user_id, date)
        date: activity.date,
        name: activity.name,
        activity_type: activity.activity_type,
        distance: activity.distance,
        duration: activity.duration,
        average_pace: activity.average_pace,
        average_speed: activity.average_speed,
        calories: activity.calories,
        climb: activity.climb,
        gps_file: activity.gps_file,
        track_points: normalized_tps,
    }
}

/// Preserve the original `Activity` UUID when converting from a parsed row.
///
/// The Runkeeper CSV carries its own UUID on column 0; callers that need to
/// retain that ID (e.g. the existing upload handler) should store the UUID
/// separately and restore it after normalisation.
#[allow(dead_code)]
pub fn original_id(activity: &Activity) -> Uuid {
    activity.id
}

/// Build a map of `gps_file → NormalizedActivity` from a slice of parsed
/// activities and a map of `gps_file → Vec<TrackPoint>`.
///
/// Activities with no matching GPX entry get an empty `track_points` vec.
#[allow(dead_code)]
pub fn build_normalized_batch(
    activities: Vec<Activity>,
    mut trackpoints_map: HashMap<Uuid, Vec<TrackPoint>>,
) -> Vec<(Uuid, NormalizedActivity)> {
    activities
        .into_iter()
        .map(|a| {
            let id = a.id;
            let tps = trackpoints_map.remove(&id).unwrap_or_default();
            (id, from_parsed(a, tps))
        })
        .collect()
}
