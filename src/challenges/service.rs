/// Business logic for the challenges domain.
///
/// No SQL here – all persistence goes through `repository`.
use std::collections::HashMap;

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::activities;
use crate::error::AppError;

use super::models::{
    ActivateChallengeRequest, AddRequirementRequest, Challenge, ChallengeDetail,
    ChallengeSummary, CreateChallengeRequest, CreateWorkoutRequest, ListChallengesParams,
    ListPublicChallengesParams, OptInRequest, ParticipantsResponse, ReorderWorkoutRequest,
    UpdateChallengeRequest, UpdateWorkoutRequest, WorkoutRequirement, WorkoutWithDetails,
};
use super::progression::{self, ProgressionTrigger};
use super::repository;
use super::status::ChallengeStatus;
use super::plan_generator;

// ─── Challenge CRUD ───────────────────────────────────────────────────────────

pub async fn list_challenges(
    db: &PgPool,
    params: ListChallengesParams,
) -> Result<Vec<ChallengeSummary>, AppError> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).max(0);

    // Flaw 5: run lazy transitions first (may flip active → expired) then re-fetch with counts.
    let challenges =
        repository::find_challenges_by_user(db, params.user_id, limit, offset).await?;
    run_lazy_transitions(db, challenges).await?;

    repository::find_challenges_with_summary(db, params.user_id, limit, offset).await
}

pub async fn get_challenge_detail(
    db: &PgPool,
    challenge_id: Uuid,
) -> Result<ChallengeDetail, AppError> {
    // Apply lazy transitions and get the (potentially updated) challenge.
    let raw = repository::find_challenge_by_id(db, challenge_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let mut transitioned = run_lazy_transitions(db, vec![raw]).await?;
    let challenge = transitioned.pop().ok_or(AppError::NotFound)?;

    let workouts = repository::find_workouts_by_challenge(db, challenge_id).await?;

    let workout_ids: Vec<Uuid> = workouts.iter().map(|w| w.id).collect();

    // Batch-load requirements and links in two round-trips.
    let all_requirements =
        repository::find_requirements_for_workouts(db, &workout_ids).await?;
    let all_links = repository::find_links_for_workouts(db, &workout_ids).await?;

    // Group by workout_id for O(1) assembly.
    let mut req_map: HashMap<Uuid, Vec<WorkoutRequirement>> = HashMap::new();
    for r in all_requirements {
        req_map.entry(r.challenge_workout_id).or_default().push(r);
    }
    let link_map: HashMap<Uuid, _> = all_links
        .into_iter()
        .map(|l| (l.challenge_workout_id, l))
        .collect();

    // Collect linked activity IDs so we can evaluate requirements.
    let activity_ids: Vec<Uuid> = link_map
        .values()
        .filter_map(|l| l.activity_id)
        .collect();
    let activity_map =
        activities::repository::find_activities_by_ids(db, &activity_ids).await?;

    // Assemble WorkoutWithDetails in position order, tracking the previous
    // linked activity so faster_than_previous can be evaluated correctly.
    let mut prev_linked_activity_id: Option<Uuid> = None;
    let mut workouts_with_details: Vec<WorkoutWithDetails> =
        Vec::with_capacity(workouts.len());

    for w in workouts {
        let requirements = req_map.remove(&w.id).unwrap_or_default();
        let link = link_map.get(&w.id).cloned();

        let prev_act = prev_linked_activity_id.and_then(|id| activity_map.get(&id));

        let state = progression::evaluate_workout_state(
            &requirements,
            link.as_ref(),
            &activity_map,
            &challenge,
            prev_act,
        );

        // Advance tracker only for completed workouts.
        if state == super::models::WorkoutState::Completed {
            prev_linked_activity_id = link.as_ref().and_then(|l| l.activity_id);
        }

        workouts_with_details.push(WorkoutWithDetails {
            workout: w,
            requirements,
            link,
            state,
        });
    }

    let participant_count = if challenge.is_public && challenge.parent_challenge_id.is_none() {
        Some(repository::find_participant_count(db, challenge_id).await?)
    } else {
        None
    };

    Ok(ChallengeDetail {
        challenge,
        workouts: workouts_with_details,
        participant_count,
    })
}

pub async fn create_challenge(
    db: &PgPool,
    req: CreateChallengeRequest,
) -> Result<Challenge, AppError> {
    req.validate().map_err(AppError::BadRequest)?;
    repository::insert_challenge(
        db,
        req.user_id,
        req.name.trim(),
        req.description.as_deref(),
        req.is_recurring.unwrap_or(false),
        req.recurrence_period.as_deref(),
        req.started_at,
        req.ends_at,
        ChallengeStatus::Draft,
        false,
        None,
    )
    .await
}

pub async fn update_challenge(
    db: &PgPool,
    challenge_id: Uuid,
    req: UpdateChallengeRequest,
) -> Result<Challenge, AppError> {
    // Validate name if supplied.
    if let Some(ref name) = req.name {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::BadRequest("Challenge name must not be empty".into()));
        }
        if name.len() > 200 {
            return Err(AppError::BadRequest(
                "Challenge name must be 200 characters or fewer".into(),
            ));
        }
    }

    // Validate ends_at > started_at when both are being changed together.
    if let (Some(started), Some(ends)) = (req.started_at, req.ends_at) {
        if ends <= started {
            return Err(AppError::BadRequest("ends_at must be after started_at".into()));
        }
    }

    // Load the current challenge to enforce the status machine lock.
    let old = repository::find_challenge_by_id(db, challenge_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let effective = ChallengeStatus::effective(old.status, old.started_at, old.ends_at);
    if effective.is_locked() {
        return Err(AppError::BadRequest(
            "Challenge is locked and cannot be modified once active or expired".into(),
        ));
    }

    repository::update_challenge(
        db,
        challenge_id,
        req.name.as_deref(),
        req.description.as_deref(),
        req.is_recurring,
        req.recurrence_period.as_deref(),
        req.started_at,
        req.ends_at,
        None,
        None,
    )
    .await?
    .ok_or(AppError::NotFound)
}

pub async fn delete_challenge(db: &PgPool, challenge_id: Uuid) -> Result<(), AppError> {
    let deleted = repository::delete_challenge(db, challenge_id).await?;
    if deleted {
        Ok(())
    } else {
        Err(AppError::NotFound)
    }
}

// ─── Workout CRUD ─────────────────────────────────────────────────────────────

pub async fn add_workout(
    db: &PgPool,
    challenge_id: Uuid,
    req: CreateWorkoutRequest,
) -> Result<crate::challenges::models::ChallengeWorkout, AppError> {
    req.validate().map_err(AppError::BadRequest)?;

    // Verify challenge exists and is not locked.
    let challenge = repository::find_challenge_by_id(db, challenge_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let effective =
        ChallengeStatus::effective(challenge.status, challenge.started_at, challenge.ends_at);
    if effective.is_locked() {
        return Err(AppError::BadRequest(
            "Challenge is locked and cannot be modified once active or expired".into(),
        ));
    }

    repository::insert_workout(
        db,
        challenge_id,
        req.name.trim(),
        req.description.as_deref(),
        req.position,
    )
    .await
}

pub async fn update_workout(
    db: &PgPool,
    workout_id: Uuid,
    req: UpdateWorkoutRequest,
) -> Result<crate::challenges::models::ChallengeWorkout, AppError> {
    if let Some(ref name) = req.name {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::BadRequest("Workout name must not be empty".into()));
        }
        if name.len() > 200 {
            return Err(AppError::BadRequest(
                "Workout name must be 200 characters or fewer".into(),
            ));
        }
    }

    let workout = repository::find_workout_by_id(db, workout_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let challenge = repository::find_challenge_by_id(db, workout.challenge_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let effective =
        ChallengeStatus::effective(challenge.status, challenge.started_at, challenge.ends_at);
    if effective.is_locked() {
        return Err(AppError::BadRequest(
            "Challenge is locked and cannot be modified once active or expired".into(),
        ));
    }

    repository::update_workout(
        db,
        workout_id,
        req.name.as_deref(),
        req.description.as_deref(),
    )
    .await?
    .ok_or(AppError::NotFound)
}

pub async fn reorder_workout(
    db: &PgPool,
    workout_id: Uuid,
    req: ReorderWorkoutRequest,
) -> Result<crate::challenges::models::ChallengeWorkout, AppError> {
    if req.new_position < 1 {
        return Err(AppError::BadRequest("Position must be >= 1".into()));
    }

    let workout = repository::find_workout_by_id(db, workout_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let challenge = repository::find_challenge_by_id(db, workout.challenge_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let effective =
        ChallengeStatus::effective(challenge.status, challenge.started_at, challenge.ends_at);
    if effective.is_locked() {
        return Err(AppError::BadRequest(
            "Challenge is locked and cannot be modified once active or expired".into(),
        ));
    }

    repository::reorder_workout(db, workout_id, req.new_position)
        .await?
        .ok_or(AppError::NotFound)
}

pub async fn delete_workout(db: &PgPool, workout_id: Uuid) -> Result<(), AppError> {
    let workout = repository::find_workout_by_id(db, workout_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let challenge = repository::find_challenge_by_id(db, workout.challenge_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let effective =
        ChallengeStatus::effective(challenge.status, challenge.started_at, challenge.ends_at);
    if effective.is_locked() {
        return Err(AppError::BadRequest(
            "Challenge is locked and cannot be modified once active or expired".into(),
        ));
    }

    let deleted = repository::delete_workout(db, workout_id).await?;
    if deleted {
        Ok(())
    } else {
        Err(AppError::NotFound)
    }
}

// ─── Requirements ─────────────────────────────────────────────────────────────

pub async fn add_requirement(
    db: &PgPool,
    workout_id: Uuid,
    req: AddRequirementRequest,
) -> Result<WorkoutRequirement, AppError> {
    // Verify workout exists and challenge is not locked.
    let workout = repository::find_workout_by_id(db, workout_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let challenge = repository::find_challenge_by_id(db, workout.challenge_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let effective =
        ChallengeStatus::effective(challenge.status, challenge.started_at, challenge.ends_at);
    if effective.is_locked() {
        return Err(AppError::BadRequest(
            "Challenge is locked and cannot be modified once active or expired".into(),
        ));
    }

    let params = req
        .params
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

    repository::insert_requirement(db, workout_id, req.requirement_type, req.value, &params)
        .await
}

pub async fn delete_requirement(db: &PgPool, requirement_id: Uuid) -> Result<(), AppError> {
    let requirement = repository::find_requirement_by_id(db, requirement_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let workout = repository::find_workout_by_id(db, requirement.challenge_workout_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let challenge = repository::find_challenge_by_id(db, workout.challenge_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let effective =
        ChallengeStatus::effective(challenge.status, challenge.started_at, challenge.ends_at);
    if effective.is_locked() {
        return Err(AppError::BadRequest(
            "Challenge is locked and cannot be modified once active or expired".into(),
        ));
    }

    let deleted = repository::delete_requirement(db, requirement_id).await?;
    if deleted {
        Ok(())
    } else {
        Err(AppError::NotFound)
    }
}

// ─── Public challenge lifecycle ───────────────────────────────────────────────

/// Transition a challenge from Draft → PendingActivation.
/// Optionally mark it public at the same time.
pub async fn activate_challenge(
    db: &PgPool,
    challenge_id: Uuid,
    req: ActivateChallengeRequest,
) -> Result<Challenge, AppError> {
    let challenge = repository::find_challenge_by_id(db, challenge_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if challenge.status != ChallengeStatus::Draft {
        return Err(AppError::BadRequest(
            "Only Draft challenges can be activated".into(),
        ));
    }

    let is_public = req.is_public.unwrap_or(false);
    repository::update_challenge(
        db,
        challenge_id,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(ChallengeStatus::PendingActivation),
        Some(is_public),
    )
    .await?
    .ok_or(AppError::NotFound)
}

/// Return publicly visible challenges (active or pending_activation) with workout
/// and participant counts in a single query.
pub async fn get_public_challenges(
    db: &PgPool,
    params: ListPublicChallengesParams,
) -> Result<Vec<ChallengeSummary>, AppError> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).max(0);
    repository::find_public_challenges_with_summary(db, limit, offset).await
}

/// Clone a public challenge for `req.user_id` (opt-in).
pub async fn opt_in_challenge(
    db: &PgPool,
    source_id: Uuid,
    req: OptInRequest,
) -> Result<Challenge, AppError> {
    let source = repository::find_challenge_by_id(db, source_id)
        .await?
        .ok_or(AppError::NotFound)?;

    // Source must be public and reachable.
    if !source.is_public {
        return Err(AppError::BadRequest("Challenge is not public".into()));
    }
    let eff = ChallengeStatus::effective(source.status, source.started_at, source.ends_at);
    if eff == ChallengeStatus::Expired {
        return Err(AppError::BadRequest(
            "Cannot opt in to an expired challenge".into(),
        ));
    }

    // Prevent duplicate opt-ins.
    if repository::check_existing_opt_in(db, source_id, req.user_id).await? {
        return Err(AppError::BadRequest(
            "User has already opted in to this challenge".into(),
        ));
    }

    // Clone inherits the same status so it immediately mirrors the source lifecycle.
    // For evergreen templates (started_at = NULL), set started_at to now() so the
    // progression engine evaluates the clone immediately after opt-in (Bug 3 fix).
    let override_started_at = if source.started_at.is_none() {
        Some(Utc::now())
    } else {
        None
    };
    repository::clone_challenge(db, source_id, req.user_id, eff, override_started_at).await
}

/// Return participant count + clone list for a public challenge.
pub async fn get_participants(
    db: &PgPool,
    challenge_id: Uuid,
    params: ListPublicChallengesParams,
) -> Result<ParticipantsResponse, AppError> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).max(0);
    let count = repository::find_participant_count(db, challenge_id).await?;
    let participants =
        repository::find_participants(db, challenge_id, limit, offset).await?;
    Ok(ParticipantsResponse { count, participants })
}

// ─── Training Plan Generation ─────────────────────────────────────────────────

pub async fn generate_challenge(
    db: &PgPool,
    req: super::models::GenerateChallengeRequest,
) -> Result<Challenge, AppError> {
    let (description, workouts) = plan_generator::generate_plan(&req);
    let weeks = req.weeks.unwrap_or(match req.goal_type.as_str() {
        "5k_improvement" => 6,
        _ => 12,
    });
    let challenge_name = req.name.clone().unwrap_or_else(|| match req.goal_type.as_str() {
        "5k_improvement" => "5 km Improvement Plan".to_string(),
        _ => "Half Marathon Training Plan".to_string(),
    });
    let ends_at = chrono::Utc::now() + chrono::Duration::weeks(weeks as i64);

    let mut tx = db.begin().await.map_err(AppError::from)?;

    let challenge = sqlx::query_as::<_, Challenge>(
        "INSERT INTO challenges (user_id, name, description, status, is_public, started_at, ends_at)
         VALUES ($1, $2, $3, 'active', false, now(), $4)
         RETURNING *",
    )
    .bind(req.user_id)
    .bind(&challenge_name)
    .bind(&description)
    .bind(ends_at)
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::from)?;

    for w in &workouts {
        let cw = sqlx::query_as::<_, super::models::ChallengeWorkout>(
            "INSERT INTO challenge_workouts (challenge_id, position, name, description)
             VALUES ($1, $2, $3, $4)
             RETURNING *",
        )
        .bind(challenge.id)
        .bind(w.position)
        .bind(&w.name)
        .bind(&w.description)
        .fetch_one(&mut *tx)
        .await
        .map_err(AppError::from)?;

        for r in &w.requirements {
            sqlx::query(
                "INSERT INTO challenge_workout_requirements \
                 (challenge_workout_id, requirement_type, value, params) \
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(cw.id)
            .bind(r.requirement_type)
            .bind(r.value)
            .bind(&r.params)
            .execute(&mut *tx)
            .await
            .map_err(AppError::from)?;
        }
    }

    tx.commit().await.map_err(AppError::from)?;
    Ok(challenge)
}

// ─── Private helpers ──────────────────────────────────────────────────────────

/// Apply time-based lazy transitions to a batch of challenges and return the
/// refreshed rows from the database.
///
/// Any challenges that transition to `active` get a `ChallengeActivated`
/// progression trigger fired (best-effort — failures are logged, not fatal).
async fn run_lazy_transitions(
    db: &PgPool,
    challenges: Vec<Challenge>,
) -> Result<Vec<Challenge>, AppError> {
    if challenges.is_empty() {
        return Ok(challenges);
    }
    let ids: Vec<Uuid> = challenges.iter().map(|c| c.id).collect();
    let activated_ids = repository::apply_lazy_transitions(db, &ids).await?;
    for cid in activated_ids {
        if let Err(e) = progression::handle(
            db,
            ProgressionTrigger::ChallengeActivated { challenge_id: cid },
        )
        .await
        {
            tracing::warn!("Progression failed after lazy activation of {cid}: {e}");
        }
    }
    repository::find_challenges_by_ids(db, &ids).await
}
