/// The `ActivitySource` trait — the single contract all data-source adapters must satisfy.
///
/// New adapters (Strava, Garmin, …) implement this trait. The ingestion pipeline
/// only depends on the trait, not on any concrete adapter.
use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

use crate::error::AppError;

use super::normalized::NormalizedActivity;

#[allow(dead_code)]
#[async_trait]
pub trait ActivitySource {
    /// Fetch all activities for the given user that occurred *after* `since`.
    ///
    /// `since = DateTime::<Utc>::from_timestamp(0, 0)` means "fetch everything".
    async fn fetch_activities(
        &self,
        user_id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<Vec<NormalizedActivity>, AppError>;
}
