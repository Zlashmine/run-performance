/// File parsing for the activities domain.
///
/// Both parsing functions are **synchronous** — they do only CPU work (no I/O).
/// Callers must not `.await` them.  The `#[allow(dead_code)]` on `clean_gpx_data`
/// is intentional: the hack is kept in one place and documented here.
use chrono::{DateTime, Utc};
use gpx::Waypoint;
use std::num::ParseFloatError;
use uuid::Uuid;

use super::models::{Activity, TrackPoint};

/// Compute the great-circle distance in metres between two WGS-84 points
/// using the Haversine formula.  Pure math — no I/O, no allocation.
pub fn haversine_distance_m(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const R: f64 = 6_371_000.0; // Earth radius in metres
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    R * c
}

/// Parse one CSV row from `cardioActivities.csv` into an `Activity`.
///
/// Expected column order (14 fields, 0-indexed):
///  0=id, 1=date, 2=activity_type, 3=name, 4=distance, 5=duration,
///  6=average_pace, 7=average_speed, 8=calories, 9=climb, …, 13=gps_file
pub fn parse_csv_row(row: &str, user_id: Uuid) -> Result<Activity, String> {
    let parts: Vec<&str> = row.split(',').collect();

    if parts.len() != 14 {
        return Err(format!("Expected 14 columns, got {}", parts.len()));
    }

    Ok(Activity {
        id: Uuid::parse_str(parts[0]).map_err(|e| e.to_string())?,
        user_id,
        date: chrono::NaiveDateTime::parse_from_str(parts[1], "%Y-%m-%d %H:%M:%S")
            .map_err(|e| e.to_string())?,
        activity_type: parts[2].to_string(),
        name: parts[3].to_string(),
        distance: parts[4]
            .parse()
            .map_err(|e: ParseFloatError| e.to_string())?,
        duration: parts[5].to_string(),
        average_pace: parts[6]
            .replace(":", ".")
            .parse()
            .map_err(|e: ParseFloatError| e.to_string())?,
        average_speed: parts[7]
            .parse()
            .map_err(|e: ParseFloatError| e.to_string())?,
        calories: parts[8]
            .parse()
            .map_err(|e: ParseFloatError| e.to_string())?,
        climb: parts[9]
            .parse()
            .map_err(|e: ParseFloatError| e.to_string())?,
        gps_file: parts[13].to_string(),
        source: "runkeeper".to_string(),
        external_id: None,
    })
}

/// Parse GPX bytes for a single activity into a list of `TrackPoint`s.
///
/// Each point's `speed` (m/s) is computed from the great-circle distance and
/// time delta between consecutive points.  The last point copies the speed of
/// its predecessor.  Points with zero time delta get `speed = None`.
///
/// # GPX quirk
/// Some Garmin exports include a non-standard creator attribute on line 11
/// (0-indexed line 10) that the `gpx` crate cannot parse.  `clean_gpx_data`
/// strips that line before parsing — this is a known workaround and must be
/// kept until the upstream import source is changed.
pub fn parse_gpx(data: &[u8], activity_id: Uuid) -> Result<Vec<TrackPoint>, String> {
    let cursor = clean_gpx_data(data)?;

    let gpx = gpx::read(cursor).map_err(|e| format!("Error reading GPX data: {}", e))?;
    let mut track_points = Vec::new();

    for track in gpx.tracks {
        for segment in track.segments {
            for waypoint in segment.points {
                let tp = waypoint_to_trackpoint(waypoint, activity_id)?;
                track_points.push(tp);
            }
        }
    }

    // Compute pairwise speed (forward: point i → point i+1).
    let n = track_points.len();
    if n >= 2 {
        for i in 0..n - 1 {
            let dist = haversine_distance_m(
                track_points[i].latitude,
                track_points[i].longitude,
                track_points[i + 1].latitude,
                track_points[i + 1].longitude,
            );
            let dt = (track_points[i + 1].time - track_points[i].time)
                .num_milliseconds() as f64
                / 1000.0;
            track_points[i].speed = if dt > 0.0 { Some(dist / dt) } else { None };
        }
        // Last point copies speed from its predecessor.
        let prev_speed = track_points[n - 2].speed;
        track_points[n - 1].speed = prev_speed;
    }

    Ok(track_points)
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn clean_gpx_data(data: &[u8]) -> Result<std::io::Cursor<String>, String> {
    let content = std::str::from_utf8(data).map_err(|e| e.to_string())?;
    let cleaned: String = content
        .lines()
        .enumerate()
        .filter_map(|(i, line)| if i == 10 { None } else { Some(line) })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(std::io::Cursor::new(cleaned))
}

fn waypoint_to_trackpoint(waypoint: Waypoint, activity_id: Uuid) -> Result<TrackPoint, String> {
    // gpx crate exposes time as `time::OffsetDateTime`; convert to chrono.
    let time: DateTime<Utc> = waypoint
        .time
        .map(|t| {
            let unix = t.format().unwrap_or_default();
            // Fastest route: re-parse the ISO-8601 string gpx already formatted.
            unix.parse::<DateTime<Utc>>().unwrap_or_else(|_| Utc::now())
        })
        .unwrap_or_else(Utc::now);

    Ok(TrackPoint {
        id: Some(Uuid::new_v4()),
        activity_id,
        latitude: waypoint.point().y(),
        longitude: waypoint.point().x(),
        elevation: waypoint.elevation.unwrap_or(0.0) as f32,
        time,
        speed: None,
    })
}
