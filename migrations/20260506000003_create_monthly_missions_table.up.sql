CREATE TABLE monthly_missions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    month_start DATE NOT NULL,
    mission_type VARCHAR(64) NOT NULL,
    title VARCHAR(128) NOT NULL,
    description TEXT NOT NULL,
    target_value DOUBLE PRECISION NOT NULL,
    current_value DOUBLE PRECISION NOT NULL DEFAULT 0,
    xp_reward INT NOT NULL DEFAULT 300,
    completed_at TIMESTAMPTZ,
    rerolled BOOLEAN NOT NULL DEFAULT false,
    is_boss BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, month_start, mission_type)
);

CREATE INDEX idx_monthly_missions_user_month ON monthly_missions(user_id, month_start);

-- Partial index for fast history queries (only completed missions, descending order)
CREATE INDEX idx_monthly_missions_completed ON monthly_missions(user_id, completed_at DESC)
    WHERE completed_at IS NOT NULL;

-- Composite index on trackpoints required for GPS exploration queries
-- (DISTINCT ON with ORDER BY activity_id, time). The existing single-column
-- idx_trackpoints_activity_id is insufficient for DISTINCT ON ordering.
CREATE INDEX idx_trackpoints_activity_time ON trackpoints(activity_id, time);
