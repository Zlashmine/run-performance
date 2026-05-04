/// Lifecycle status for a challenge.
///
/// Stored as TEXT in the `challenges.status` column.
/// sqlx integration mirrors `requirement_type.rs`: TEXT-backed manual
/// Type/Encode/Decode so no PostgreSQL ENUM migration is needed.
use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgArgumentBuffer, PgTypeInfo, PgValueRef};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChallengeStatus {
    Draft,
    PendingActivation,
    Active,
    Expired,
}

impl ChallengeStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft             => "draft",
            Self::PendingActivation => "pending_activation",
            Self::Active            => "active",
            Self::Expired           => "expired",
        }
    }

    /// Compute the effective status given stored status + current wall-clock time.
    pub fn effective(
        stored: Self,
        started_at: Option<DateTime<Utc>>,
        ends_at: Option<DateTime<Utc>>,
    ) -> Self {
        let now = Utc::now();
        match stored {
            Self::PendingActivation => {
                if started_at.is_some_and(|s| s <= now) {
                    Self::Active
                } else {
                    stored
                }
            }
            Self::Active => {
                if ends_at.is_some_and(|e| e <= now) {
                    Self::Expired
                } else {
                    stored
                }
            }
            _ => stored,
        }
    }

    /// Is editing blocked for this status?
    pub fn is_locked(self) -> bool {
        !matches!(self, Self::Draft)
    }

    /// Should the progression engine process this challenge?
    pub fn should_run_progression(self) -> bool {
        matches!(self, Self::Active)
    }
}

impl fmt::Display for ChallengeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ChallengeStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft"              => Ok(Self::Draft),
            "pending_activation" => Ok(Self::PendingActivation),
            "active"             => Ok(Self::Active),
            "expired"            => Ok(Self::Expired),
            other                => Err(format!("unknown challenge status: {other}")),
        }
    }
}

// ─── sqlx TEXT-backed integration (same boilerplate as RequirementType) ──────

impl sqlx::Type<sqlx::Postgres> for ChallengeStatus {
    fn type_info() -> PgTypeInfo {
        <String as sqlx::Type<sqlx::Postgres>>::type_info()
    }
    fn compatible(ty: &PgTypeInfo) -> bool {
        <String as sqlx::Type<sqlx::Postgres>>::compatible(ty)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Postgres> for ChallengeStatus {
    fn decode(
        value: PgValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let s = <&str as sqlx::Decode<sqlx::Postgres>>::decode(value)?;
        s.parse().map_err(|e: String| e.into())
    }
}

impl sqlx::Encode<'_, sqlx::Postgres> for ChallengeStatus {
    fn encode_by_ref(
        &self,
        buf: &mut PgArgumentBuffer,
    ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        let s = self.as_str();
        <&str as sqlx::Encode<sqlx::Postgres>>::encode_by_ref(&s, buf)
    }
}
