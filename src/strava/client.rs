/// HTTP client wrapper for the Strava API.
///
/// Handles:
///   - Automatic token refresh when `expires_at` is within 5 minutes.
///   - Rate-limit header inspection (logged; caller may inspect too).
///   - All outgoing requests use Bearer auth.
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

// ─── Public types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct StravaClient {
    pub http:          Client,
    pub client_id:     String,
    pub client_secret: String,
}

/// Response from `POST /oauth/token` (both `authorization_code` and `refresh_token`).
#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token:  String,
    pub refresh_token: String,
    /// Unix epoch seconds at which the access token expires.
    pub expires_at:    i64,
    pub athlete:       Option<AthleteInfo>,
}

#[derive(Debug, Deserialize)]
pub struct AthleteInfo {
    pub id:         i64,
    pub firstname:  Option<String>,
    pub lastname:   Option<String>,
}

/// A full `DetailedActivity` as returned by `GET /activities/{id}`.
#[derive(Debug, Deserialize)]
pub struct StravaDetailedActivity {
    pub id:                   i64,
    pub name:                 String,
    pub sport_type:           String,
    pub start_date:           String, // ISO 8601 UTC
    pub elapsed_time:         i64,    // seconds
    pub distance:             f64,    // metres
    pub total_elevation_gain: f64,    // metres
    pub calories:             Option<f64>,
    pub average_speed:        f64,    // m/s
}

/// A `SummaryActivity` as returned by `GET /athlete/activities`.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct StravaSummaryActivity {
    pub id:         i64,
    pub sport_type: String,
}

/// Streams response from `GET /activities/{id}/streams?key_by_type=true`.
#[derive(Debug, Deserialize)]
pub struct StreamSet {
    pub latlng:          Option<StreamData<[f64; 2]>>,
    pub altitude:        Option<StreamData<f64>>,
    pub time:            Option<StreamData<i64>>,
    pub velocity_smooth: Option<StreamData<f64>>,
}

#[derive(Debug, Deserialize)]
pub struct StreamData<T> {
    pub data: Vec<T>,
}

// ─── StravaClient implementation ──────────────────────────────────────────────

impl StravaClient {
    pub fn new() -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build reqwest client");

        let client_id     = std::env::var("STRAVA_CLIENT_ID").unwrap_or_default();
        let client_secret = std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default();

        StravaClient { http, client_id, client_secret }
    }

    /// Exchange an authorization code for tokens.
    pub async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<TokenResponse, AppError> {
        let params = [
            ("client_id",     self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("code",          code),
            ("grant_type",    "authorization_code"),
            ("redirect_uri",  redirect_uri),
        ];

        let resp = self
            .http
            .post("https://www.strava.com/api/v3/oauth/token")
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Strava token exchange failed: {e}");
                AppError::Internal
            })?;

        if !resp.status().is_success() {
            tracing::error!("Strava token exchange HTTP {}", resp.status());
            return Err(AppError::Internal);
        }

        resp.json::<TokenResponse>().await.map_err(|e| {
            tracing::error!("Strava token response parse failed: {e}");
            AppError::Internal
        })
    }

    /// Refresh an access token using a refresh token.
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<TokenResponse, AppError> {
        let params = [
            ("client_id",     self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type",    "refresh_token"),
        ];

        let resp = self
            .http
            .post("https://www.strava.com/api/v3/oauth/token")
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Strava token refresh failed: {e}");
                AppError::Internal
            })?;

        if !resp.status().is_success() {
            tracing::error!("Strava token refresh HTTP {}", resp.status());
            return Err(AppError::Internal);
        }

        resp.json::<TokenResponse>().await.map_err(|e| {
            tracing::error!("Strava refresh response parse failed: {e}");
            AppError::Internal
        })
    }

    /// Revoke the user's authorization (POST /oauth/deauthorize).
    pub async fn deauthorize(&self, access_token: &str) -> Result<(), AppError> {
        let params = [("access_token", access_token)];
        let _ = self
            .http
            .post("https://www.strava.com/oauth/deauthorize")
            .form(&params)
            .send()
            .await; // best-effort; ignore errors
        Ok(())
    }

    /// Get a valid access token for the given user, refreshing if necessary.
    ///
    /// Always persists a freshly refreshed token back to `strava_tokens`.
    pub async fn get_valid_token(
        &self,
        db: &PgPool,
        user_id: Uuid,
    ) -> Result<String, AppError> {
        #[derive(sqlx::FromRow)]
        struct TokenRow { access_token: String, refresh_token: String, expires_at: i64 }
        let row = sqlx::query_as::<_, TokenRow>(
            "SELECT access_token, refresh_token, expires_at
             FROM strava_tokens WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_optional(db)
        .await
        .map_err(AppError::from)?
        .ok_or(AppError::NotFound)?;

        let now = Utc::now().timestamp();
        // Refresh if within 5 minutes of expiry.
        if row.expires_at - now < 300 {
            let refreshed = self.refresh_token(&row.refresh_token).await?;
            sqlx::query(
                "UPDATE strava_tokens
                 SET access_token = $1, refresh_token = $2, expires_at = $3, updated_at = now()
                 WHERE user_id = $4"
            )
            .bind(&refreshed.access_token)
            .bind(&refreshed.refresh_token)
            .bind(refreshed.expires_at)
            .bind(user_id)
            .execute(db)
            .await
            .map_err(AppError::from)?;
            return Ok(refreshed.access_token);
        }

        Ok(row.access_token)
    }

    // ── Data-fetch helpers ────────────────────────────────────────────────────

    /// `GET /athlete/activities?after={since}&per_page=50&page={page}`
    pub async fn list_activities(
        &self,
        token: &str,
        after: i64,
        page: u32,
    ) -> Result<Vec<StravaSummaryActivity>, AppError> {
        let resp = self
            .http
            .get("https://www.strava.com/api/v3/athlete/activities")
            .bearer_auth(token)
            .query(&[
                ("after",    after.to_string()),
                ("per_page", "50".to_string()),
                ("page",     page.to_string()),
            ])
            .send()
            .await
            .map_err(|e| { tracing::error!("list_activities error: {e}"); AppError::Internal })?;

        if resp.status().as_u16() == 429 {
            tracing::warn!("Strava rate limit hit on list_activities");
            return Err(AppError::Internal);
        }
        if !resp.status().is_success() {
            tracing::error!("list_activities HTTP {}", resp.status());
            return Err(AppError::Internal);
        }

        resp.json::<Vec<StravaSummaryActivity>>().await.map_err(|e| {
            tracing::error!("list_activities parse error: {e}");
            AppError::Internal
        })
    }

    /// `GET /activities/{id}` — full DetailedActivity
    pub async fn get_activity(
        &self,
        token: &str,
        activity_id: i64,
    ) -> Result<StravaDetailedActivity, AppError> {
        let url = format!("https://www.strava.com/api/v3/activities/{}", activity_id);
        let resp = self
            .http
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| { tracing::error!("get_activity error: {e}"); AppError::Internal })?;

        if !resp.status().is_success() {
            tracing::error!("get_activity {} HTTP {}", activity_id, resp.status());
            return Err(AppError::Internal);
        }

        resp.json::<StravaDetailedActivity>().await.map_err(|e| {
            tracing::error!("get_activity parse error: {e}");
            AppError::Internal
        })
    }

    /// `GET /activities/{id}/streams?keys=latlng,altitude,time,velocity_smooth&key_by_type=true`
    pub async fn get_streams(
        &self,
        token: &str,
        activity_id: i64,
    ) -> Result<StreamSet, AppError> {
        let url = format!(
            "https://www.strava.com/api/v3/activities/{}/streams",
            activity_id
        );
        let resp = self
            .http
            .get(&url)
            .bearer_auth(token)
            .query(&[
                ("keys",         "latlng,altitude,time,velocity_smooth"),
                ("key_by_type",  "true"),
            ])
            .send()
            .await
            .map_err(|e| { tracing::error!("get_streams error: {e}"); AppError::Internal })?;

        if resp.status().as_u16() == 404 {
            // Activity has no streams (e.g. manually entered) — return empty set.
            return Ok(StreamSet { latlng: None, altitude: None, time: None, velocity_smooth: None });
        }
        if !resp.status().is_success() {
            tracing::warn!("get_streams {} HTTP {}", activity_id, resp.status());
            return Ok(StreamSet { latlng: None, altitude: None, time: None, velocity_smooth: None });
        }

        resp.json::<StreamSet>().await.map_err(|e| {
            tracing::error!("get_streams parse error: {e}");
            // Non-fatal — return empty streams rather than aborting the whole sync.
            AppError::Internal
        })
    }
}

// ─── Strava → NormalizedActivity conversion ────────────────────────────────

use crate::sync::normalized::{NormalizedActivity, NormalizedTrackPoint};

/// Normalise a `StravaDetailedActivity` + its `StreamSet` into a `NormalizedActivity`.
pub fn normalize(
    detail: &StravaDetailedActivity,
    streams: StreamSet,
    start_dt: chrono::DateTime<Utc>,
) -> NormalizedActivity {
    // ── Scalar fields ──────────────────────────────────────────────────────
    let distance_km  = (detail.distance / 1000.0) as f32;
    let avg_speed_kh = (detail.average_speed * 3.6) as f32;
    let activity_type = sport_type_to_activity_type(&detail.sport_type);

    // Pace (min/km) — meaningful only for running-like activities.
    let average_pace = if detail.average_speed > 0.01 && activity_type == "Running" {
        (1000.0 / detail.average_speed / 60.0) as f32
    } else {
        0.0
    };

    let duration = seconds_to_hms(detail.elapsed_time);

    // ── Track points ────────────────────────────────────────────────────────
    let empty_latlng:  Vec<[f64; 2]> = vec![];
    let empty_alt:     Vec<f64>      = vec![];
    let empty_time:    Vec<i64>      = vec![];
    let empty_vel:     Vec<f64>      = vec![];

    let latlng  = streams.latlng          .as_ref().map(|s| s.data.as_slice()).unwrap_or(&empty_latlng);
    let alt     = streams.altitude        .as_ref().map(|s| s.data.as_slice()).unwrap_or(&empty_alt);
    let times   = streams.time            .as_ref().map(|s| s.data.as_slice()).unwrap_or(&empty_time);
    let vels    = streams.velocity_smooth .as_ref().map(|s| s.data.as_slice()).unwrap_or(&empty_vel);

    let n = latlng.len();
    let mut track_points = Vec::with_capacity(n);
    for i in 0..n {
        let lat = latlng[i][0];
        let lon = latlng[i][1];
        let elevation = alt.get(i).copied().unwrap_or(0.0) as f32;
        let t_offset  = times.get(i).copied().unwrap_or(0);
        let speed     = vels.get(i).copied();
        let time      = start_dt + chrono::Duration::seconds(t_offset);
        track_points.push(NormalizedTrackPoint { latitude: lat, longitude: lon, elevation, time, speed });
    }

    NormalizedActivity {
        source:         "strava".to_string(),
        external_id:    Some(detail.id.to_string()),
        date:           start_dt.naive_utc(),
        name:           detail.name.clone(),
        activity_type,
        distance:       distance_km,
        duration,
        average_pace,
        average_speed:  avg_speed_kh,
        calories:       detail.calories.unwrap_or(0.0) as f32,
        climb:          detail.total_elevation_gain as f32,
        gps_file:       "".to_string(),
        track_points,
    }
}

fn sport_type_to_activity_type(sport_type: &str) -> String {
    match sport_type {
        "Run" | "TrailRun" | "VirtualRun" => "Running",
        "Ride" | "VirtualRide" | "MountainBikeRide" | "GravelRide" | "EBikeRide" => "Cycling",
        "Swim" => "Swimming",
        "Walk" | "Hike" => "Walking",
        _ => "Other",
    }
    .to_string()
}

fn seconds_to_hms(seconds: i64) -> String {
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

// ─── Strava token DB helpers ────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
pub struct StravaTokenRow {
    pub user_id:             Uuid,
    pub strava_athlete_id:   i64,
    pub strava_athlete_name: String,
    pub access_token:        String,
    pub refresh_token:       String,
    pub expires_at:          i64,
    pub last_synced_at:      Option<chrono::DateTime<Utc>>,
}

pub async fn upsert_tokens(
    db: &PgPool,
    user_id: Uuid,
    token_resp: &TokenResponse,
) -> Result<(), AppError> {
    let athlete_id = token_resp.athlete.as_ref().map(|a| a.id).unwrap_or(0);
    let athlete_name = token_resp.athlete.as_ref().map(|a| {
        format!(
            "{} {}",
            a.firstname.as_deref().unwrap_or(""),
            a.lastname.as_deref().unwrap_or("")
        )
        .trim()
        .to_string()
    }).unwrap_or_default();

    sqlx::query(
        r#"
        INSERT INTO strava_tokens
            (user_id, strava_athlete_id, strava_athlete_name, access_token, refresh_token, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (user_id) DO UPDATE
            SET strava_athlete_id   = EXCLUDED.strava_athlete_id,
                strava_athlete_name = EXCLUDED.strava_athlete_name,
                access_token        = EXCLUDED.access_token,
                refresh_token       = EXCLUDED.refresh_token,
                expires_at          = EXCLUDED.expires_at,
                updated_at          = NOW()
        "#
    )
    .bind(user_id)
    .bind(athlete_id)
    .bind(athlete_name)
    .bind(&token_resp.access_token)
    .bind(&token_resp.refresh_token)
    .bind(token_resp.expires_at)
    .execute(db)
    .await
    .map_err(AppError::from)?;
    Ok(())
}

pub async fn delete_tokens(db: &PgPool, user_id: Uuid) -> Result<(), AppError> {
    sqlx::query("DELETE FROM strava_tokens WHERE user_id = $1")
        .bind(user_id)
        .execute(db)
        .await
        .map_err(AppError::from)?;
    Ok(())
}

pub async fn find_user_by_athlete_id(db: &PgPool, athlete_id: i64) -> Result<Option<Uuid>, AppError> {
    sqlx::query_scalar::<_, Uuid>(
        "SELECT user_id FROM strava_tokens WHERE strava_athlete_id = $1"
    )
    .bind(athlete_id)
    .fetch_optional(db)
    .await
    .map_err(AppError::from)
}
