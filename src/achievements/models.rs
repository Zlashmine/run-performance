use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct AchievementDefinition {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub xp_reward: i32,
    pub rarity: String,
    pub category: String,
    pub is_secret: bool,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct UserAchievement {
    pub id: Uuid,
    pub user_id: Uuid,
    pub achievement_id: Uuid,
    pub activity_id: Option<Uuid>,
    pub unlocked_at: DateTime<Utc>,
}

/// Joined view sent to the client.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AchievementWithStatus {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub xp_reward: i32,
    pub rarity: String,
    pub category: String,
    pub is_secret: bool,
    pub unlocked: bool,
    pub unlocked_at: Option<DateTime<Utc>>,
    pub activity_id: Option<Uuid>,
}

/// Compact summary included in the upload response.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UnlockedAchievementSummary {
    pub slug: String,
    pub name: String,
    pub icon: String,
    pub rarity: String,
    pub xp_reward: i32,
}
