CREATE TABLE weekly_missions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    week_start DATE NOT NULL,
    mission_type VARCHAR(32) NOT NULL CHECK (mission_type IN (
        'run_distance_km',
        'run_count',
        'beat_last_week_km',
        'run_on_skipped_day',
        'run_sub_pace',
        'longest_single_run'
    )),
    title VARCHAR(128) NOT NULL,
    description TEXT NOT NULL,
    target_value DOUBLE PRECISION NOT NULL,
    current_value DOUBLE PRECISION NOT NULL DEFAULT 0,
    xp_reward INT NOT NULL DEFAULT 100,
    completed_at TIMESTAMPTZ,
    rerolled BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, week_start, mission_type)
);

CREATE INDEX idx_weekly_missions_user_week ON weekly_missions(user_id, week_start);
