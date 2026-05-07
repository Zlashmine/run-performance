/// SQL layer for the challenges domain.
///
/// All queries live here – no SQL in services or handlers.
use chrono::{DateTime, Utc};
use sqlx::{PgPool, QueryBuilder};
use uuid::Uuid;

use crate::error::AppError;

use super::models::{
    Challenge, ChallengeWorkout, ChallengeSummary, WorkoutLink, WorkoutRequirement,
};
use super::requirement_type::RequirementType;
use super::status::ChallengeStatus;

// ─── Challenges ──────────────────────────────────────────────────────────────

pub async fn find_challenges_by_user(
    db: &PgPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<Challenge>, AppError> {
    sqlx::query_as::<_, Challenge>(
        "SELECT * FROM challenges WHERE user_id = $1
         ORDER BY created_at DESC
         LIMIT $2 OFFSET $3",
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}

pub async fn find_challenge_by_id(
    db: &PgPool,
    challenge_id: Uuid,
) -> Result<Option<Challenge>, AppError> {
    sqlx::query_as::<_, Challenge>("SELECT * FROM challenges WHERE id = $1")
        .bind(challenge_id)
        .fetch_optional(db)
        .await
        .map_err(AppError::from)
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_challenge(
    db: &PgPool,
    user_id: Uuid,
    name: &str,
    description: Option<&str>,
    is_recurring: bool,
    recurrence_period: Option<&str>,
    started_at: Option<DateTime<Utc>>,
    ends_at: Option<DateTime<Utc>>,
    status: ChallengeStatus,
    is_public: bool,
    parent_challenge_id: Option<Uuid>,
) -> Result<Challenge, AppError> {
    sqlx::query_as::<_, Challenge>(
        "INSERT INTO challenges
             (user_id, name, description, is_recurring, recurrence_period,
              started_at, ends_at, status, is_public, parent_challenge_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         RETURNING *",
    )
    .bind(user_id)
    .bind(name)
    .bind(description)
    .bind(is_recurring)
    .bind(recurrence_period)
    .bind(started_at)
    .bind(ends_at)
    .bind(status)
    .bind(is_public)
    .bind(parent_challenge_id)
    .fetch_one(db)
    .await
    .map_err(AppError::from)
}

#[allow(clippy::too_many_arguments)]
pub async fn update_challenge(
    db: &PgPool,
    challenge_id: Uuid,
    name: Option<&str>,
    description: Option<&str>,
    is_recurring: Option<bool>,
    recurrence_period: Option<&str>,
    started_at: Option<DateTime<Utc>>,
    ends_at: Option<DateTime<Utc>>,
    status: Option<ChallengeStatus>,
    is_public: Option<bool>,
) -> Result<Option<Challenge>, AppError> {
    sqlx::query_as::<_, Challenge>(
        "UPDATE challenges SET
             name              = COALESCE($2, name),
             description       = COALESCE($3, description),
             is_recurring      = COALESCE($4, is_recurring),
             recurrence_period = COALESCE($5, recurrence_period),
             started_at        = COALESCE($6, started_at),
             ends_at           = COALESCE($7, ends_at),
             status            = COALESCE($8, status),
             is_public         = COALESCE($9, is_public),
             updated_at        = now()
         WHERE id = $1
         RETURNING *",
    )
    .bind(challenge_id)
    .bind(name)
    .bind(description)
    .bind(is_recurring)
    .bind(recurrence_period)
    .bind(started_at)
    .bind(ends_at)
    .bind(status)
    .bind(is_public)
    .fetch_optional(db)
    .await
    .map_err(AppError::from)
}

#[allow(dead_code)]
pub async fn set_challenge_started(
    db: &PgPool,
    challenge_id: Uuid,
) -> Result<Option<Challenge>, AppError> {
    sqlx::query_as::<_, Challenge>(
        "UPDATE challenges SET started_at = now(), updated_at = now()
         WHERE id = $1 AND started_at IS NULL
         RETURNING *",
    )
    .bind(challenge_id)
    .fetch_optional(db)
    .await
    .map_err(AppError::from)
}

pub async fn delete_challenge(db: &PgPool, challenge_id: Uuid) -> Result<bool, AppError> {
    let result =
        sqlx::query("DELETE FROM challenges WHERE id = $1")
            .bind(challenge_id)
            .execute(db)
            .await
            .map_err(AppError::from)?;
    Ok(result.rows_affected() > 0)
}

// ─── Workouts ────────────────────────────────────────────────────────────────

pub async fn find_workouts_by_challenge(
    db: &PgPool,
    challenge_id: Uuid,
) -> Result<Vec<ChallengeWorkout>, AppError> {
    sqlx::query_as::<_, ChallengeWorkout>(
        "SELECT * FROM challenge_workouts WHERE challenge_id = $1 ORDER BY position ASC",
    )
    .bind(challenge_id)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}

pub async fn find_workout_by_id(
    db: &PgPool,
    workout_id: Uuid,
) -> Result<Option<ChallengeWorkout>, AppError> {
    sqlx::query_as::<_, ChallengeWorkout>(
        "SELECT * FROM challenge_workouts WHERE id = $1",
    )
    .bind(workout_id)
    .fetch_optional(db)
    .await
    .map_err(AppError::from)
}

/// Insert a new workout slot.
/// If `position` is None, appends after the current maximum position.
pub async fn insert_workout(
    db: &PgPool,
    challenge_id: Uuid,
    name: &str,
    description: Option<&str>,
    position: Option<i32>,
) -> Result<ChallengeWorkout, AppError> {
    // Determine position: explicit or MAX+1.
    let pos: i32 = if let Some(p) = position {
        p
    } else {
        let max: Option<i32> = sqlx::query_scalar(
            "SELECT MAX(position) FROM challenge_workouts WHERE challenge_id = $1",
        )
        .bind(challenge_id)
        .fetch_one(db)
        .await
        .map_err(AppError::from)?;
        max.unwrap_or(0) + 1
    };

    sqlx::query_as::<_, ChallengeWorkout>(
        "INSERT INTO challenge_workouts (challenge_id, position, name, description)
         VALUES ($1, $2, $3, $4)
         RETURNING *",
    )
    .bind(challenge_id)
    .bind(pos)
    .bind(name)
    .bind(description)
    .fetch_one(db)
    .await
    .map_err(AppError::from)
}

pub async fn update_workout(
    db: &PgPool,
    workout_id: Uuid,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<Option<ChallengeWorkout>, AppError> {
    sqlx::query_as::<_, ChallengeWorkout>(
        "UPDATE challenge_workouts SET
             name        = COALESCE($2, name),
             description = COALESCE($3, description),
             updated_at  = now()
         WHERE id = $1
         RETURNING *",
    )
    .bind(workout_id)
    .bind(name)
    .bind(description)
    .fetch_optional(db)
    .await
    .map_err(AppError::from)
}

/// Atomically move a workout to `new_position` within its challenge,
/// shifting other workouts to preserve a gap-free sequence.
///
/// Uses a DEFERRABLE UNIQUE constraint on (challenge_id, position) so
/// intermediate states during reordering don't trigger constraint violations.
pub async fn reorder_workout(
    db: &PgPool,
    workout_id: Uuid,
    new_position: i32,
) -> Result<Option<ChallengeWorkout>, AppError> {
    let mut tx = db.begin().await.map_err(AppError::from)?;

    // Fetch the workout to reorder.
    let workout = sqlx::query_as::<_, ChallengeWorkout>(
        "SELECT * FROM challenge_workouts WHERE id = $1",
    )
    .bind(workout_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::from)?;

    let Some(workout) = workout else {
        return Ok(None);
    };

    let old_position = workout.position;
    let challenge_id = workout.challenge_id;

    if old_position == new_position {
        tx.rollback().await.map_err(AppError::from)?;
        return Ok(Some(workout));
    }

    // Shift neighbours to make room (deferred constraint keeps this safe).
    if new_position < old_position {
        // Moving up: shift range [new_position, old_position-1] down by 1.
        sqlx::query(
            "UPDATE challenge_workouts SET position = position + 1, updated_at = now()
             WHERE challenge_id = $1 AND position >= $2 AND position < $3",
        )
        .bind(challenge_id)
        .bind(new_position)
        .bind(old_position)
        .execute(&mut *tx)
        .await
        .map_err(AppError::from)?;
    } else {
        // Moving down: shift range [old_position+1, new_position] up by 1.
        sqlx::query(
            "UPDATE challenge_workouts SET position = position - 1, updated_at = now()
             WHERE challenge_id = $1 AND position > $2 AND position <= $3",
        )
        .bind(challenge_id)
        .bind(old_position)
        .bind(new_position)
        .execute(&mut *tx)
        .await
        .map_err(AppError::from)?;
    }

    // Place the workout at the new position.
    let result = sqlx::query_as::<_, ChallengeWorkout>(
        "UPDATE challenge_workouts SET position = $2, updated_at = now()
         WHERE id = $1
         RETURNING *",
    )
    .bind(workout_id)
    .bind(new_position)
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::from)?;

    tx.commit().await.map_err(AppError::from)?;
    Ok(result)
}

pub async fn delete_workout(db: &PgPool, workout_id: Uuid) -> Result<bool, AppError> {
    let result = sqlx::query("DELETE FROM challenge_workouts WHERE id = $1")
        .bind(workout_id)
        .execute(db)
        .await
        .map_err(AppError::from)?;
    Ok(result.rows_affected() > 0)
}

// ─── Requirements ─────────────────────────────────────────────────────────────

pub async fn find_requirements_for_workouts(
    db: &PgPool,
    workout_ids: &[Uuid],
) -> Result<Vec<WorkoutRequirement>, AppError> {
    if workout_ids.is_empty() {
        return Ok(vec![]);
    }
    sqlx::query_as::<_, WorkoutRequirement>(
        "SELECT * FROM challenge_workout_requirements
         WHERE challenge_workout_id = ANY($1)",
    )
    .bind(workout_ids)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}

pub async fn insert_requirement(
    db: &PgPool,
    workout_id: Uuid,
    requirement_type: RequirementType,
    value: Option<f64>,
    params: &serde_json::Value,
) -> Result<WorkoutRequirement, AppError> {
    sqlx::query_as::<_, WorkoutRequirement>(
        "INSERT INTO challenge_workout_requirements
             (challenge_workout_id, requirement_type, value, params)
         VALUES ($1, $2, $3, $4)
         RETURNING *",
    )
    .bind(workout_id)
    .bind(requirement_type)
    .bind(value)
    .bind(params)
    .fetch_one(db)
    .await
    .map_err(AppError::from)
}

pub async fn delete_requirement(
    db: &PgPool,
    requirement_id: Uuid,
) -> Result<bool, AppError> {
    let result =
        sqlx::query("DELETE FROM challenge_workout_requirements WHERE id = $1")
            .bind(requirement_id)
            .execute(db)
            .await
            .map_err(AppError::from)?;
    Ok(result.rows_affected() > 0)
}

// ─── Links ────────────────────────────────────────────────────────────────────

pub async fn find_links_for_workouts(
    db: &PgPool,
    workout_ids: &[Uuid],
) -> Result<Vec<WorkoutLink>, AppError> {
    if workout_ids.is_empty() {
        return Ok(vec![]);
    }
    sqlx::query_as::<_, WorkoutLink>(
        "SELECT * FROM challenge_workout_links WHERE challenge_workout_id = ANY($1)",
    )
    .bind(workout_ids)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}

#[allow(dead_code)]
pub async fn find_link_by_workout(
    db: &PgPool,
    workout_id: Uuid,
) -> Result<Option<WorkoutLink>, AppError> {
    sqlx::query_as::<_, WorkoutLink>(
        "SELECT * FROM challenge_workout_links WHERE challenge_workout_id = $1",
    )
    .bind(workout_id)
    .fetch_optional(db)
    .await
    .map_err(AppError::from)
}

#[allow(dead_code)]
pub async fn update_link_state(
    db: &PgPool,
    workout_id: Uuid,
    state: &str,
) -> Result<Option<WorkoutLink>, AppError> {
    sqlx::query_as::<_, WorkoutLink>(
        "UPDATE challenge_workout_links SET state = $2
         WHERE challenge_workout_id = $1
         RETURNING *",
    )
    .bind(workout_id)
    .bind(state)
    .fetch_optional(db)
    .await
    .map_err(AppError::from)
}

/// Delete all link rows for every workout belonging to a challenge.
/// Accepts any sqlx executor (pool or transaction) so the caller controls
/// transaction boundaries.
pub async fn delete_links_for_challenge<'c, E>(
    executor: E,
    workout_ids: &[Uuid],
) -> Result<(), AppError>
where
    E: sqlx::Executor<'c, Database = sqlx::Postgres>,
{
    if workout_ids.is_empty() {
        return Ok(());
    }
    sqlx::query(
        "DELETE FROM challenge_workout_links
         WHERE challenge_workout_id = ANY($1)",
    )
    .bind(workout_ids)
    .execute(executor)
    .await
    .map_err(AppError::from)?;
    Ok(())
}

/// Bulk-insert (workout_id, activity_id) pairs as 'completed' links.
/// Uses a single INSERT … VALUES (…), (…) round-trip.
pub async fn bulk_insert_links(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    pairs: &[(Uuid, Uuid)],
) -> Result<(), AppError> {
    if pairs.is_empty() {
        return Ok(());
    }
    let mut qb = QueryBuilder::new(
        "INSERT INTO challenge_workout_links \
         (challenge_workout_id, activity_id, state) ",
    );
    qb.push_values(pairs, |mut b, (wid, aid)| {
        b.push_bind(*wid).push_bind(*aid).push_bind("completed");
    });
    qb.build().execute(&mut **tx).await.map_err(AppError::from)?;
    Ok(())
}

/// Return all challenges that are in 'active' status for a given user.
/// Used by the progression engine to find which challenges to re-evaluate
/// when activities are uploaded.
pub async fn find_active_challenges_for_user(
    db: &PgPool,
    user_id: Uuid,
) -> Result<Vec<Challenge>, AppError> {
    sqlx::query_as::<_, Challenge>(
        "SELECT * FROM challenges
         WHERE user_id = $1
           AND status = 'active'
           AND started_at IS NOT NULL",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}

pub async fn find_challenges_by_ids(
    db: &PgPool,
    ids: &[Uuid],
) -> Result<Vec<Challenge>, AppError> {
    if ids.is_empty() {
        return Ok(vec![]);
    }
    sqlx::query_as::<_, Challenge>(
        "SELECT * FROM challenges WHERE id = ANY($1) ORDER BY created_at DESC",
    )
    .bind(ids)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}

/// Lazily transition challenges:
/// - `pending_activation` → `active`  when `started_at <= now()`
/// - `active`             → `expired` when `ends_at   <= now()`
///
/// Returns the IDs of challenges that were transitioned to `active`
/// so callers can fire the `ChallengeActivated` progression trigger.
pub async fn apply_lazy_transitions(
    db: &PgPool,
    challenge_ids: &[Uuid],
) -> Result<Vec<Uuid>, AppError> {
    if challenge_ids.is_empty() {
        return Ok(vec![]);
    }
    let activated: Vec<Uuid> = sqlx::query_scalar(
        "UPDATE challenges
         SET status = 'active', updated_at = now()
         WHERE id = ANY($1)
           AND status = 'pending_activation'
           AND started_at IS NOT NULL
           AND started_at <= now()
         RETURNING id",
    )
    .bind(challenge_ids)
    .fetch_all(db)
    .await
    .map_err(AppError::from)?;

    sqlx::query(
        "UPDATE challenges
         SET status = 'expired', updated_at = now()
         WHERE id = ANY($1)
           AND status = 'active'
           AND ends_at IS NOT NULL
           AND ends_at <= now()",
    )
    .bind(challenge_ids)
    .execute(db)
    .await
    .map_err(AppError::from)?;

    Ok(activated)
}

// ─── Summary queries (list endpoints) ────────────────────────────────────────

/// Returns challenges for a user with progress counts in a single query.
/// The caller must run `run_lazy_transitions` first so status is up to date.
pub async fn find_challenges_with_summary(
    db: &PgPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<ChallengeSummary>, AppError> {
    sqlx::query_as::<_, ChallengeSummary>(
        "SELECT
            c.*,
            COUNT(DISTINCT cw.id)  AS workout_count,
            -- All link rows have state='completed'; COUNT on links = completed count.
            COUNT(DISTINCT cwl.id) AS completed_count,
            (
                SELECT r.params->>'activity_type_is'
                FROM challenge_workout_requirements r
                JOIN challenge_workouts cw2 ON cw2.id = r.challenge_workout_id
                WHERE cw2.challenge_id = c.id
                  AND r.requirement_type = 'activity_type_is'
                LIMIT 1
            ) AS primary_activity_type,
            NULL::bigint AS participant_count
         FROM challenges c
         LEFT JOIN challenge_workouts      cw  ON cw.challenge_id         = c.id
         LEFT JOIN challenge_workout_links cwl ON cwl.challenge_workout_id = cw.id
         WHERE c.user_id = $1
         GROUP BY c.id
         ORDER BY c.created_at DESC
         LIMIT $2 OFFSET $3",
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}

/// Returns public challenges with workout count and participant count in a single query.
pub async fn find_public_challenges_with_summary(
    db: &PgPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<ChallengeSummary>, AppError> {
    sqlx::query_as::<_, ChallengeSummary>(
        "SELECT
            c.*,
            COUNT(DISTINCT cw.id)    AS workout_count,
            0::bigint                AS completed_count,
            (
                SELECT r.params->>'activity_type_is'
                FROM challenge_workout_requirements r
                JOIN challenge_workouts cw2 ON cw2.id = r.challenge_workout_id
                WHERE cw2.challenge_id = c.id
                  AND r.requirement_type = 'activity_type_is'
                LIMIT 1
            ) AS primary_activity_type,
            COUNT(DISTINCT clones.id) AS participant_count
         FROM challenges c
         LEFT JOIN challenge_workouts cw     ON cw.challenge_id           = c.id
         LEFT JOIN challenges         clones ON clones.parent_challenge_id = c.id
         WHERE c.is_public = TRUE
           AND c.status IN ('active', 'pending_activation')
         GROUP BY c.id
         ORDER BY participant_count DESC, c.created_at DESC
         LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}

pub async fn find_participant_count(
    db: &PgPool,
    challenge_id: Uuid,
) -> Result<i64, AppError> {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM challenges WHERE parent_challenge_id = $1",
    )
    .bind(challenge_id)
    .fetch_one(db)
    .await
    .map_err(AppError::from)
}

pub async fn find_participants(
    db: &PgPool,
    challenge_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<Challenge>, AppError> {
    sqlx::query_as::<_, Challenge>(
        "SELECT * FROM challenges
         WHERE parent_challenge_id = $1
         ORDER BY created_at ASC
         LIMIT $2 OFFSET $3",
    )
    .bind(challenge_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}

pub async fn check_existing_opt_in(
    db: &PgPool,
    challenge_id: Uuid,
    user_id: Uuid,
) -> Result<bool, AppError> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM challenges
         WHERE parent_challenge_id = $1 AND user_id = $2",
    )
    .bind(challenge_id)
    .bind(user_id)
    .fetch_one(db)
    .await
    .map_err(AppError::from)?;
    Ok(count > 0)
}

/// Clone a public challenge for a new user.
///
/// Copies: challenge row, workout rows, requirement rows.
/// Does NOT copy `challenge_workout_links` (those are earned, not configured).
pub async fn clone_challenge(
    db: &PgPool,
    source_id: Uuid,
    new_user_id: Uuid,
    initial_status: ChallengeStatus,
    override_started_at: Option<DateTime<Utc>>,
) -> Result<Challenge, AppError> {
    let mut tx = db.begin().await.map_err(AppError::from)?;

    let source = sqlx::query_as::<_, Challenge>(
        "SELECT * FROM challenges WHERE id = $1",
    )
    .bind(source_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(AppError::from)?
    .ok_or(AppError::NotFound)?;

    let clone_started_at = override_started_at.or(source.started_at);

    let new_challenge = sqlx::query_as::<_, Challenge>(
        "INSERT INTO challenges
             (user_id, name, description, is_recurring, recurrence_period,
              started_at, ends_at, status, is_public, parent_challenge_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, false, $9)
         RETURNING *",
    )
    .bind(new_user_id)
    .bind(&source.name)
    .bind(&source.description)
    .bind(source.is_recurring)
    .bind(&source.recurrence_period)
    .bind(clone_started_at)
    .bind(source.ends_at)
    .bind(initial_status)
    .bind(source_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(AppError::from)?;

    let source_workouts = sqlx::query_as::<_, ChallengeWorkout>(
        "SELECT * FROM challenge_workouts WHERE challenge_id = $1 ORDER BY position ASC",
    )
    .bind(source_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(AppError::from)?;

    for workout in &source_workouts {
        let new_workout = sqlx::query_as::<_, ChallengeWorkout>(
            "INSERT INTO challenge_workouts (challenge_id, position, name, description)
             VALUES ($1, $2, $3, $4)
             RETURNING *",
        )
        .bind(new_challenge.id)
        .bind(workout.position)
        .bind(&workout.name)
        .bind(&workout.description)
        .fetch_one(&mut *tx)
        .await
        .map_err(AppError::from)?;

        let reqs = sqlx::query_as::<_, WorkoutRequirement>(
            "SELECT * FROM challenge_workout_requirements WHERE challenge_workout_id = $1",
        )
        .bind(workout.id)
        .fetch_all(&mut *tx)
        .await
        .map_err(AppError::from)?;

        for req in reqs {
            sqlx::query(
                "INSERT INTO challenge_workout_requirements
                     (challenge_workout_id, requirement_type, value, params)
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(new_workout.id)
            .bind(req.requirement_type)
            .bind(req.value)
            .bind(&req.params)
            .execute(&mut *tx)
            .await
            .map_err(AppError::from)?;
        }
    }

    tx.commit().await.map_err(AppError::from)?;
    Ok(new_challenge)
}

/// Return challenges that may need lazy status transitions for a given user.
/// Used by the progression engine before re-evaluating after activity uploads.
pub async fn find_transitioning_challenges_for_user(
    db: &PgPool,
    user_id: Uuid,
) -> Result<Vec<Challenge>, AppError> {
    sqlx::query_as::<_, Challenge>(
        "SELECT * FROM challenges
         WHERE user_id = $1
           AND status IN ('pending_activation', 'active')",
    )
    .bind(user_id)
    .fetch_all(db)
    .await
    .map_err(AppError::from)
}

pub async fn find_requirement_by_id(
    db: &PgPool,
    requirement_id: Uuid,
) -> Result<Option<WorkoutRequirement>, AppError> {
    sqlx::query_as::<_, WorkoutRequirement>(
        "SELECT * FROM challenge_workout_requirements WHERE id = $1",
    )
    .bind(requirement_id)
    .fetch_optional(db)
    .await
    .map_err(AppError::from)
}

// ─── Leaderboard ─────────────────────────────────────────────────────────────

pub async fn get_leaderboard(
    db: &PgPool,
    challenge_id: Uuid,
) -> Result<super::models::LeaderboardResponse, AppError> {
    // Fetch name + public gate fields
    let (name, is_public, parent_id): (String, bool, Option<Uuid>) =
        sqlx::query_as::<_, (String, bool, Option<Uuid>)>(
            "SELECT name, is_public, parent_challenge_id FROM challenges WHERE id = $1",
        )
        .bind(challenge_id)
        .fetch_optional(db)
        .await
        .map_err(AppError::from)?
        .ok_or(AppError::NotFound)?;

    if !is_public && parent_id.is_none() {
        return Err(AppError::BadRequest(
            "Leaderboard is only available for public challenges".to_string(),
        ));
    }

    // CTE: rank all participants (original owner + clones) by completed workouts.
    // The "root" is either the challenge itself (if it owns the workout definitions)
    // or its parent (if it is a clone).
    let rows: Vec<(Uuid, String, i64, i64)> = sqlx::query_as::<_, (Uuid, String, i64, i64)>(
        r#"
        WITH template AS (
            SELECT CASE
                       WHEN parent_challenge_id IS NULL THEN id
                       ELSE parent_challenge_id
                   END AS root_id
            FROM challenges
            WHERE id = $1
        ),
        family AS (
            SELECT c.id AS challenge_id, c.user_id
            FROM challenges c, template t
            WHERE c.id = t.root_id
               OR c.parent_challenge_id = t.root_id
        ),
        total_workouts AS (
            SELECT COUNT(*) AS cnt
            FROM challenge_workouts
            WHERE challenge_id = (SELECT root_id FROM template)
        ),
        completed_per_user AS (
            SELECT f.user_id,
                   COUNT(DISTINCT cwl.id) AS completed
            FROM family f
            JOIN challenge_workouts cw ON cw.challenge_id = f.challenge_id
            LEFT JOIN challenge_workout_links cwl
                ON cwl.challenge_workout_id = cw.id
               AND cwl.state = 'completed'
            GROUP BY f.user_id
        )
        SELECT cpu.user_id,
               u.email,
               cpu.completed,
               (SELECT cnt FROM total_workouts) AS total
        FROM completed_per_user cpu
        JOIN users u ON u.id = cpu.user_id
        ORDER BY cpu.completed DESC
        "#,
    )
    .bind(challenge_id)
    .fetch_all(db)
    .await
    .map_err(AppError::from)?;

    let total_participants = rows.len() as i64;
    let entries: Vec<super::models::LeaderboardEntry> = rows
        .into_iter()
        .enumerate()
        .map(|(i, (user_id, email, completed, total))| {
            let display_name = email
                .split('@')
                .next()
                .unwrap_or(&email)
                .to_string();
            let completion_percent = if total > 0 {
                (completed as f64 / total as f64 * 100.0).min(100.0)
            } else {
                0.0
            };
            super::models::LeaderboardEntry {
                rank: (i as i64) + 1,
                user_id,
                display_name,
                completed_workouts: completed,
                total_workouts: total,
                completion_percent,
            }
        })
        .collect();

    Ok(super::models::LeaderboardResponse {
        challenge_id,
        challenge_name: name,
        total_participants,
        entries,
    })
}
