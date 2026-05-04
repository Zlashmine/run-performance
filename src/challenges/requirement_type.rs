/// Strongly-typed enum for workout requirement kinds.
///
/// Replaces the former `VALID_REQUIREMENT_TYPES: &[&str]` approach.
/// Serde automatically rejects unknown variants, so no manual validation
/// is required any more.
///
/// # sqlx integration
///
/// The `challenge_workout_requirements.requirement_type` column is `TEXT`.
/// The manual `Type / Encode / Decode` implementations below map each
/// variant to/from its snake_case string exactly as stored in the DB.
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgArgumentBuffer, PgTypeInfo, PgValueRef};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RequirementType {
    PaceFasterThan,
    DistanceLongerThan,
    DaysSinceChallengeStart,
    DaysSinceFirstWorkout,
    FasterThanPrevious,
    DurationLongerThan,
    PaceSlowerThan,
    ClimbAtLeast,
    CaloriesAtLeast,
    LongerThanPrevious,
    DistanceIncreasedByPercent,
    DaysAfterPreviousWorkout,
    SpeedAtLeast,
    ActivityTypeIs,
}

impl RequirementType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PaceFasterThan => "pace_faster_than",
            Self::DistanceLongerThan => "distance_longer_than",
            Self::DaysSinceChallengeStart => "days_since_challenge_start",
            Self::DaysSinceFirstWorkout => "days_since_first_workout",
            Self::FasterThanPrevious => "faster_than_previous",
            Self::DurationLongerThan => "duration_longer_than",
            Self::PaceSlowerThan => "pace_slower_than",
            Self::ClimbAtLeast => "climb_at_least",
            Self::CaloriesAtLeast => "calories_at_least",
            Self::LongerThanPrevious => "longer_than_previous",
            Self::DistanceIncreasedByPercent => "distance_increased_by_percent",
            Self::DaysAfterPreviousWorkout => "days_after_previous_workout",
            Self::SpeedAtLeast => "speed_at_least",
            Self::ActivityTypeIs => "activity_type_is",
        }
    }
}

impl fmt::Display for RequirementType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RequirementType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pace_faster_than" => Ok(Self::PaceFasterThan),
            "distance_longer_than" => Ok(Self::DistanceLongerThan),
            "days_since_challenge_start" => Ok(Self::DaysSinceChallengeStart),
            "days_since_first_workout" => Ok(Self::DaysSinceFirstWorkout),
            "faster_than_previous" => Ok(Self::FasterThanPrevious),
            "duration_longer_than" => Ok(Self::DurationLongerThan),
            "pace_slower_than" => Ok(Self::PaceSlowerThan),
            "climb_at_least" => Ok(Self::ClimbAtLeast),
            "calories_at_least" => Ok(Self::CaloriesAtLeast),
            "longer_than_previous" => Ok(Self::LongerThanPrevious),
            "distance_increased_by_percent" => Ok(Self::DistanceIncreasedByPercent),
            "days_after_previous_workout" => Ok(Self::DaysAfterPreviousWorkout),
            "speed_at_least" => Ok(Self::SpeedAtLeast),
            "activity_type_is" => Ok(Self::ActivityTypeIs),
            other => Err(format!("unknown requirement type: {other}")),
        }
    }
}

// ─── sqlx integration ─────────────────────────────────────────────────────────

impl sqlx::Type<sqlx::Postgres> for RequirementType {
    fn type_info() -> PgTypeInfo {
        // Map to the PostgreSQL built-in TEXT type so sqlx can read/write
        // this from any TEXT column without a PostgreSQL ENUM type.
        <String as sqlx::Type<sqlx::Postgres>>::type_info()
    }

    fn compatible(ty: &PgTypeInfo) -> bool {
        <String as sqlx::Type<sqlx::Postgres>>::compatible(ty)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Postgres> for RequirementType {
    fn decode(
        value: PgValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let s = <&str as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        s.parse().map_err(|e: String| e.into())
    }
}

impl sqlx::Encode<'_, sqlx::Postgres> for RequirementType {
    fn encode_by_ref(
        &self,
        buf: &mut PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        let s = self.as_str();
        <&str as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&s, buf)
    }
}
