-- Goals: user-defined measurable targets with optional activity filters.
CREATE TABLE goals (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID        NOT NULL REFERENCES users(id),
    name          TEXT        NOT NULL,
    description   TEXT,
    timeframe     TEXT        NOT NULL CHECK (timeframe IN ('monthly', 'yearly', 'forever')),
    -- For monthly goals: 'YYYY-MM', yearly: 'YYYY', forever: ''
    period_key    TEXT        NOT NULL DEFAULT '',
    current_value DOUBLE PRECISION NOT NULL DEFAULT 0,
    target_value  DOUBLE PRECISION NOT NULL,
    completed_at  TIMESTAMPTZ,
    xp_reward     INT         NOT NULL DEFAULT 150,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Each goal has exactly one 'metric' requirement and zero or more 'filter' requirements.
CREATE TABLE goal_requirements (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    goal_id          UUID        NOT NULL REFERENCES goals(id) ON DELETE CASCADE,
    -- 'metric': the primary measured value; 'filter': activity pre-selection predicate
    category         TEXT        NOT NULL CHECK (category IN ('metric', 'filter')),
    requirement_type TEXT        NOT NULL,
    value            DOUBLE PRECISION,
    params           JSONB       NOT NULL DEFAULT '{}',
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_goals_user_id ON goals(user_id);
CREATE INDEX idx_goal_requirements_goal_id ON goal_requirements(goal_id);
