/// Server-driven challenge progression engine.
///
/// This module owns all logic for automatically matching activities to
/// battle-pass workouts. Call [`handle`] from any site that may change
/// progression state (challenge created/updated, activities uploaded).
use std::collections::{HashMap, HashSet};

use sqlx::PgPool;
use uuid::Uuid;

use crate::activities;
use crate::error::AppError;

use super::models::{Challenge, WorkoutLink, WorkoutRequirement, WorkoutState};
use super::requirement_type::RequirementType;
use super::repository;

// ─── Public trigger API ────────────────────────────────────────────────────────

/// Events that should trigger progression recalculation.
/// Add a new variant here to support new trigger sources in the future;
/// only [`handle`] needs updating.
pub enum ProgressionTrigger {
    /// A challenge's `started_at` (or `ends_at`) was added or changed.
    #[allow(dead_code)]
    ChallengeStartDateChanged { challenge_id: Uuid },
    /// New activities were uploaded for a user.
    ActivitiesUploaded { user_id: Uuid },
    /// A challenge just transitioned from PendingActivation → Active (lazy).
    ChallengeActivated { challenge_id: Uuid },
}

/// Dispatch a trigger event to the appropriate recalculation path.
///
/// For `ActivitiesUploaded`, activities are loaded **once** for the user
/// and reused across all of the user's active challenges for efficiency.
pub async fn handle(db: &PgPool, trigger: ProgressionTrigger) -> Result<(), AppError> {
    match trigger {
        ProgressionTrigger::ChallengeStartDateChanged { challenge_id } => {
            recalculate_one(db, challenge_id).await?;
        }

        ProgressionTrigger::ChallengeActivated { challenge_id } => {
            recalculate_one(db, challenge_id).await?;
        }

        ProgressionTrigger::ActivitiesUploaded { user_id } => {
            // First, apply any pending lazy transitions for the user's challenges.
            let transitioning =
                repository::find_transitioning_challenges_for_user(db, user_id).await?;
            if !transitioning.is_empty() {
                let ids: Vec<Uuid> = transitioning.iter().map(|c| c.id).collect();
                let _activated = repository::apply_lazy_transitions(db, &ids).await?;
            }
            let challenges =
                repository::find_active_challenges_for_user(db, user_id).await?;
            if challenges.is_empty() {
                return Ok(());
            }

            // Load activities once, from the earliest challenge start date.
            let earliest_from = challenges
                .iter()
                .filter_map(|c| c.started_at)
                .min()
                .expect("find_active_challenges_for_user only returns challenges with started_at");

            let all_activities =
                activities::repository::find_activities_by_user_from(
                    db, user_id, earliest_from, None,
                )
                .await?;

            for challenge in &challenges {
                if let Err(e) =
                    recalculate_with_activities(db, challenge, &all_activities).await
                {
                    // Progression failure must not abort the upload response.
                    tracing::warn!(
                        "Progression failed for challenge {}: {e}",
                        challenge.id
                    );
                }
            }
        }
    }
    Ok(())
}

// ─── Core recalculation ────────────────────────────────────────────────────────

/// Recalculate progression for a single challenge, loading its own activity
/// window from the database.
async fn recalculate_one(db: &PgPool, challenge_id: Uuid) -> Result<usize, AppError> {
    let challenge = repository::find_challenge_by_id(db, challenge_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let Some(from) = challenge.started_at else {
        // No start date → nothing to evaluate.
        return Ok(0);
    };

    let activities = activities::repository::find_activities_by_user_from(
        db,
        challenge.user_id,
        from,
        challenge.ends_at,
    )
    .await?;

    recalculate_with_activities(db, &challenge, &activities).await
}

/// Core algorithm.
///
/// Activity list must already be sorted by date ASC and filtered to
/// `[challenge.started_at, challenge.ends_at]`.
///
/// Steps:
///  1. Acquire a per-challenge advisory lock to prevent concurrent runs.
///  2. Clear all existing links for the challenge's workouts.
///  3. Walk workouts in position order, greedily assigning the earliest
///     activity that satisfies all requirements.
///  4. Stop at the first unmatched workout; all later workouts remain
///     link-free (⇒ `NotStarted` at read time).
///  5. Bulk-insert the matched links in one round-trip.
///  6. Commit.
async fn recalculate_with_activities(
    db: &PgPool,
    challenge: &Challenge,
    all_activities: &[crate::activities::models::Activity],
) -> Result<usize, AppError> {
    let Some(from) = challenge.started_at else {
        return Ok(0);
    };

    // Skip challenges that are not in an active state.
    if !challenge.status.should_run_progression() {
        return Ok(0);
    }

    // Filter to this challenge's time window (caller may have passed a
    // broader slice when processing multiple challenges at once).
    let activities: Vec<_> = all_activities
        .iter()
        .filter(|a| {
            let dt = a.date.and_utc();
            dt >= from && challenge.ends_at.map_or(true, |end| dt <= end)
        })
        .collect();

    let workouts = repository::find_workouts_by_challenge(db, challenge.id).await?;
    if workouts.is_empty() {
        return Ok(0);
    }

    let workout_ids: Vec<Uuid> = workouts.iter().map(|w| w.id).collect();

    // Load all requirements in a single query, keyed by workout.
    let all_reqs = repository::find_requirements_for_workouts(db, &workout_ids).await?;
    let mut req_map: HashMap<Uuid, Vec<WorkoutRequirement>> = HashMap::new();
    for r in all_reqs {
        req_map.entry(r.challenge_workout_id).or_default().push(r);
    }

    // Begin transaction and acquire an advisory lock keyed on the
    // challenge ID to prevent concurrent double-recalculation.
    let mut tx = db.begin().await.map_err(AppError::from)?;

    // pg_advisory_xact_lock takes an i64; fold the 128-bit UUID into 64 bits
    // by XOR-ing the two halves. Collision probability is negligible.
    let lock_key = {
        let bytes = challenge.id.as_u128();
        ((bytes >> 64) as i64) ^ (bytes as i64)
    };
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(lock_key)
        .execute(&mut *tx)
        .await
        .map_err(AppError::from)?;

    // Clear existing links; after this point all workouts show as NotStarted.
    repository::delete_links_for_challenge(&mut *tx, &workout_ids).await?;

    // Greedy walk: assign the earliest eligible activity to each workout.
    let mut used: HashSet<Uuid> = HashSet::new();
    let mut prev_activity: Option<&crate::activities::models::Activity> = None;
    let mut links: Vec<(Uuid, Uuid)> = Vec::new();

    'outer: for workout in &workouts {
        let empty = vec![];
        let reqs = req_map.get(&workout.id).unwrap_or(&empty);

        for activity in &activities {
            if used.contains(&activity.id) {
                continue;
            }
            if evaluate_requirements(reqs, activity, challenge, prev_activity) {
                links.push((workout.id, activity.id));
                used.insert(activity.id);
                prev_activity = Some(activity);
                continue 'outer;
            }
        }
        // No matching activity found — stop. Remaining workouts stay linkless
        // which resolves to WorkoutState::NotStarted at read time.
        break;
    }

    let linked_count = links.len();
    repository::bulk_insert_links(&mut tx, &links).await?;
    tx.commit().await.map_err(AppError::from)?;

    Ok(linked_count)
}

// ─── Requirement evaluation ────────────────────────────────────────────────────

/// Re-evaluate a single workout slot for the detail view.
///
/// Returns:
/// - `NotStarted` if there is no link or no activity on the link.
/// - `Completed` if all requirements pass (or there are none).
/// - `Failed` if any requirement fails.
pub(crate) fn evaluate_workout_state(
    requirements: &[WorkoutRequirement],
    link: Option<&WorkoutLink>,
    activity_map: &HashMap<Uuid, crate::activities::models::Activity>,
    challenge: &Challenge,
    previous_activity: Option<&crate::activities::models::Activity>,
) -> WorkoutState {
    let Some(link) = link else {
        return WorkoutState::NotStarted;
    };
    let Some(activity_id) = link.activity_id else {
        return WorkoutState::NotStarted;
    };
    let Some(activity) = activity_map.get(&activity_id) else {
        return WorkoutState::NotStarted;
    };

    if requirements.is_empty() {
        return WorkoutState::Completed;
    }

    if evaluate_requirements(requirements, activity, challenge, previous_activity) {
        WorkoutState::Completed
    } else {
        WorkoutState::Failed
    }
}

/// Returns `true` if the activity satisfies all requirements on a workout.
/// An empty requirements list is trivially satisfied.
pub(crate) fn evaluate_requirements(
    requirements: &[WorkoutRequirement],
    activity: &crate::activities::models::Activity,
    challenge: &Challenge,
    previous_activity: Option<&crate::activities::models::Activity>,
) -> bool {
    if requirements.is_empty() {
        return true;
    }
    requirements
        .iter()
        .all(|req| evaluate_single_requirement(req, activity, challenge, previous_activity))
}

/// Returns `true` if the activity satisfies one requirement.
///
/// Activity numeric fields (`distance`, `average_pace`) are `f32`; we cast
/// to `f64` for comparison against `requirement.value: Option<f64>`.
fn evaluate_single_requirement(
    req: &WorkoutRequirement,
    activity: &crate::activities::models::Activity,
    challenge: &Challenge,
    previous_activity: Option<&crate::activities::models::Activity>,
) -> bool {
    match req.requirement_type {
        RequirementType::PaceFasterThan => {
            // pace is seconds-per-km: smaller value is faster.
            let threshold = req.value.unwrap_or(f64::MAX);
            (activity.average_pace as f64) < threshold
        }

        RequirementType::DistanceLongerThan => {
            let threshold = req.value.unwrap_or(0.0);
            (activity.distance as f64) > threshold
        }

        RequirementType::DaysSinceChallengeStart => {
            let Some(started_at) = challenge.started_at else {
                return false;
            };
            let start_date = started_at.date_naive();
            let activity_date = activity.date.date();
            let days = (activity_date - start_date).num_days();
            let threshold = req.value.unwrap_or(0.0) as i64;
            days >= threshold
        }

        RequirementType::DaysSinceFirstWorkout => {
            // `first_workout_date` in params is an ISO 8601 date string
            // supplied by the user at requirement-creation time.
            let first_date_str = req
                .params
                .get("first_workout_date")
                .and_then(|v| v.as_str());
            let Some(first_date_str) = first_date_str else {
                return false;
            };
            let Ok(first_date) =
                chrono::NaiveDate::parse_from_str(first_date_str, "%Y-%m-%d")
            else {
                return false;
            };
            let activity_date = activity.date.date();
            let days = (activity_date - first_date).num_days();
            let threshold = req.value.unwrap_or(0.0) as i64;
            days >= threshold
        }

        RequirementType::FasterThanPrevious => {
            // In auto-progression mode the previous workout's activity is
            // tracked dynamically by the engine — no static params needed.
            let Some(prev) = previous_activity else {
                // Workout at position 1 has no predecessor; requirement is
                // unsatisfiable. (Adding faster_than_previous to the first
                // workout is a user error the UI should prevent.)
                return false;
            };
            (activity.average_pace as f64) < (prev.average_pace as f64)
        }

        RequirementType::DurationLongerThan => {
            // duration is stored as "HH:MM:SS"
            let parts: Vec<u64> = activity
                .duration
                .split(':')
                .filter_map(|s| s.parse().ok())
                .collect();
            let total_minutes = match parts.as_slice() {
                [h, m, s] => h * 60 + m + (*s / 60),
                [m, s] => m + (*s / 60),
                _ => 0,
            };
            total_minutes as f64 > req.value.unwrap_or(0.0)
        }

        RequirementType::PaceSlowerThan => {
            let threshold = req.value.unwrap_or(0.0);
            (activity.average_pace as f64) > threshold
        }

        RequirementType::ClimbAtLeast => {
            let threshold = req.value.unwrap_or(0.0);
            (activity.climb as f64) >= threshold
        }

        RequirementType::CaloriesAtLeast => {
            let threshold = req.value.unwrap_or(0.0);
            (activity.calories as f64) >= threshold
        }

        RequirementType::LongerThanPrevious => {
            let Some(prev) = previous_activity else {
                return false;
            };
            activity.distance > prev.distance
        }

        RequirementType::DistanceIncreasedByPercent => {
            let Some(prev) = previous_activity else {
                // No previous workout at position 1 → pass automatically.
                return true;
            };
            let factor = 1.0 + req.value.unwrap_or(0.0) / 100.0;
            (activity.distance as f64) >= (prev.distance as f64) * factor
        }

        RequirementType::DaysAfterPreviousWorkout => {
            let Some(prev) = previous_activity else {
                // No previous workout at position 1 → pass automatically.
                return true;
            };
            let days = (activity.date.date() - prev.date.date()).num_days();
            days >= req.value.unwrap_or(0.0) as i64
        }

        RequirementType::SpeedAtLeast => {
            let threshold = req.value.unwrap_or(0.0);
            (activity.average_speed as f64) >= threshold
        }

        RequirementType::ActivityTypeIs => {
            let required = req
                .params
                .get("activity_type")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            activity.activity_type.to_lowercase() == required.to_lowercase()
        }
    }
}
