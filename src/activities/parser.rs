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
    })
}

/// Parse GPX bytes for a single activity into a list of `TrackPoint`s.
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
    })
}
