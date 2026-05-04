-- ── challenges ──────────────────────────────────────────────────────────────
-- A named collection of ordered workout slots that a user wants to complete.
CREATE TABLE challenges (
    id                UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id           UUID        NOT NULL,
    name              TEXT        NOT NULL,
    description       TEXT,
    is_recurring      BOOLEAN     NOT NULL DEFAULT false,
    recurrence_period TEXT,
    started_at        TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_challenges_user
        FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
);

CREATE INDEX idx_challenges_user_id
    ON challenges (user_id, created_at DESC);

-- ── challenge_workouts ───────────────────────────────────────────────────────
-- Ordered slots within a challenge. `position` is 1-based.
-- uq_challenge_workout_position is DEFERRABLE so reorder transactions
-- don't fail mid-update when target position already exists.
CREATE TABLE challenge_workouts (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    challenge_id UUID        NOT NULL,
    position     INT         NOT NULL,
    name         TEXT        NOT NULL,
    description  TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_challenge_workouts_challenge
        FOREIGN KEY (challenge_id) REFERENCES challenges (id) ON DELETE CASCADE,
    CONSTRAINT uq_challenge_workout_position
        UNIQUE (challenge_id, position) DEFERRABLE INITIALLY DEFERRED
);

CREATE INDEX idx_challenge_workouts_challenge_id
    ON challenge_workouts (challenge_id, position ASC);

-- ── challenge_workout_requirements ──────────────────────────────────────────
-- Zero or more typed requirements attached to a workout slot.
CREATE TABLE challenge_workout_requirements (
    id                   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    challenge_workout_id UUID        NOT NULL,
    requirement_type     TEXT        NOT NULL
        CHECK (requirement_type IN (
            'pace_faster_than',
            'distance_longer_than',
            'days_since_challenge_start',
            'days_since_first_workout',
            'faster_than_previous'
        )),
    value                DOUBLE PRECISION,
    params               JSONB       NOT NULL DEFAULT '{}',
    created_at           TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_requirements_workout
        FOREIGN KEY (challenge_workout_id)
            REFERENCES challenge_workouts (id) ON DELETE CASCADE
);

CREATE INDEX idx_requirements_workout_id
    ON challenge_workout_requirements (challenge_workout_id);

-- ── challenge_workout_links ──────────────────────────────────────────────────
-- Connects a workout slot to an activity the user submits as evidence.
-- `state` caches the evaluated WorkoutState for fast list queries.
CREATE TABLE challenge_workout_links (
    id                   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    challenge_workout_id UUID        NOT NULL UNIQUE,
    activity_id          UUID,
    state                TEXT        NOT NULL DEFAULT 'not_started',
    linked_at            TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_links_workout
        FOREIGN KEY (challenge_workout_id)
            REFERENCES challenge_workouts (id) ON DELETE CASCADE,
    CONSTRAINT fk_links_activity
        FOREIGN KEY (activity_id)
            REFERENCES activities (id) ON DELETE SET NULL
);

CREATE INDEX idx_links_activity_id
    ON challenge_workout_links (activity_id);
