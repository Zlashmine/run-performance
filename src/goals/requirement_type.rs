/// Strongly-typed enums for goal requirement kinds.
///
/// `GoalMetricType` — the primary measured value for a goal (exactly one per goal).
/// `GoalFilterType` — optional predicates that narrow which activities count.
///
/// Both enums use the same sqlx manual Encode/Decode pattern as
/// `challenges::requirement_type::RequirementType`, mapping each variant to its
/// snake_case TEXT representation as stored in goal_requirements.requirement_type.
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgArgumentBuffer, PgTypeInfo, PgValueRef};
use utoipa::ToSchema;

// ─── Metric types ─────────────────────────────────────────────────────────────

/// The primary metric measured by a goal.
///
/// Completion direction:
///   - FastestPace / AveragePace: complete when current <= target (lower secs/km = faster)
///   - All others: complete when current >= target
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum GoalMetricType {
    TotalDistance,    // km
    TotalDuration,    // minutes
    TotalActivities,  // count
    TotalElevation,   // metres
    TotalCalories,    // kcal
    LongestRun,       // km (single-activity max)
    FastestPace,      // secs/km (best single activity; LOWER = better)
    AveragePace,      // secs/km (mean across filtered activities; LOWER = better)
}

impl GoalMetricType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TotalDistance => "total_distance",
            Self::TotalDuration => "total_duration",
            Self::TotalActivities => "total_activities",
            Self::TotalElevation => "total_elevation",
            Self::TotalCalories => "total_calories",
            Self::LongestRun => "longest_run",
            Self::FastestPace => "fastest_pace",
            Self::AveragePace => "average_pace",
        }
    }

    /// Returns true when the goal is met given the current and target values.
    /// Pace metrics are inverted: lower secs/km means a faster pace.
    pub fn is_met(self, current: f64, target: f64) -> bool {
        match self {
            Self::FastestPace | Self::AveragePace => current > 0.0 && current <= target,
            _ => current >= target,
        }
    }
}

impl fmt::Display for GoalMetricType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for GoalMetricType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "total_distance" => Ok(Self::TotalDistance),
            "total_duration" => Ok(Self::TotalDuration),
            "total_activities" => Ok(Self::TotalActivities),
            "total_elevation" => Ok(Self::TotalElevation),
            "total_calories" => Ok(Self::TotalCalories),
            "longest_run" => Ok(Self::LongestRun),
            "fastest_pace" => Ok(Self::FastestPace),
            "average_pace" => Ok(Self::AveragePace),
            other => Err(format!("unknown GoalMetricType: {other}")),
        }
    }
}

impl sqlx::Type<sqlx::Postgres> for GoalMetricType {
    fn type_info() -> PgTypeInfo {
        <String as sqlx::Type<sqlx::Postgres>>::type_info()
    }

    fn compatible(ty: &PgTypeInfo) -> bool {
        <String as sqlx::Type<sqlx::Postgres>>::compatible(ty)
    }
}

impl sqlx::Encode<'_, sqlx::Postgres> for GoalMetricType {
    fn encode_by_ref(
        &self,
        buf: &mut PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        let s = self.as_str();
        <&str as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&s, buf)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Postgres> for GoalMetricType {
    fn decode(value: PgValueRef<'r>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let raw = <&str as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        Self::from_str(raw).map_err(|e| e.into())
    }
}

// ─── Filter types ─────────────────────────────────────────────────────────────

/// Optional activity-selection predicates.
///
/// Each filter narrows the set of activities that contribute to the metric.
/// Filters are AND-chained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum GoalFilterType {
    ActivityTypeIs,  // params.activity_type = "Running" etc.
    MinDistance,     // activity.distance >= value (km)
    MaxDistance,     // activity.distance <= value (km)
    MinDuration,     // activity.duration >= value (minutes)
    MinPace,         // activity.average_pace <= value (secs/km)
    MaxPace,         // activity.average_pace >= value (secs/km)
    MinElevation,    // activity.climb >= value (metres)
}

impl GoalFilterType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ActivityTypeIs => "activity_type_is",
            Self::MinDistance => "min_distance",
            Self::MaxDistance => "max_distance",
            Self::MinDuration => "min_duration",
            Self::MinPace => "min_pace",
            Self::MaxPace => "max_pace",
            Self::MinElevation => "min_elevation",
        }
    }
}

impl fmt::Display for GoalFilterType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for GoalFilterType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "activity_type_is" => Ok(Self::ActivityTypeIs),
            "min_distance" => Ok(Self::MinDistance),
            "max_distance" => Ok(Self::MaxDistance),
            "min_duration" => Ok(Self::MinDuration),
            "min_pace" => Ok(Self::MinPace),
            "max_pace" => Ok(Self::MaxPace),
            "min_elevation" => Ok(Self::MinElevation),
            other => Err(format!("unknown GoalFilterType: {other}")),
        }
    }
}

impl sqlx::Type<sqlx::Postgres> for GoalFilterType {
    fn type_info() -> PgTypeInfo {
        <String as sqlx::Type<sqlx::Postgres>>::type_info()
    }

    fn compatible(ty: &PgTypeInfo) -> bool {
        <String as sqlx::Type<sqlx::Postgres>>::compatible(ty)
    }
}

impl sqlx::Encode<'_, sqlx::Postgres> for GoalFilterType {
    fn encode_by_ref(
        &self,
        buf: &mut PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        let s = self.as_str();
        <&str as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&s, buf)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Postgres> for GoalFilterType {
    fn decode(value: PgValueRef<'r>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let raw = <&str as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        Self::from_str(raw).map_err(|e| e.into())
    }
}
