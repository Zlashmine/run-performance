/// Canonical representation of an activity, independent of data source.
///
/// Both the Runkeeper adapter and the Strava adapter produce `NormalizedActivity`
/// values. The ingestion pipeline (`activities::service::ingest_activities`) only
/// speaks this type, ensuring XP/achievements/PR pipelines run identically for
/// all sources.
use chrono::NaiveDateTime;

#[derive(Debug, Clone)]
pub struct NormalizedActivity {
    /// Data source identifier: `"runkeeper"` or `"strava"`.
    pub source: String,

    /// Source-specific stable ID used for deduplication.
    /// `None` for legacy Runkeeper rows (dedup falls back to `(user_id, date)`).
    pub external_id: Option<String>,

    pub date: NaiveDateTime,
    pub name: String,
    /// E.g. `"Running"`, `"Cycling"`, `"Swimming"`.
    pub activity_type: String,
    /// Kilometres.
    pub distance: f32,
    /// `HH:MM:SS` string.
    pub duration: String,
    /// Minutes per kilometre (0.0 for non-running types).
    pub average_pace: f32,
    /// Kilometres per hour.
    pub average_speed: f32,
    pub calories: f32,
    /// Metres of positive elevation gain.
    pub climb: f32,
    /// Original GPX filename (empty string when none).
    pub gps_file: String,

    /// GPS track points, if available.
    pub track_points: Vec<NormalizedTrackPoint>,
}

#[derive(Debug, Clone)]
pub struct NormalizedTrackPoint {
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: f32,
    pub time: chrono::DateTime<chrono::Utc>,
    /// Speed in m/s, if recorded by the device.
    pub speed: Option<f64>,
}
