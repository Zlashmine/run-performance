use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ActivitiesAggregation {
    pub total_activities: u32,
    pub total_distance: f32,
    pub average_pace: f32,
}
