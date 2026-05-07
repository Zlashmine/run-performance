CREATE TABLE personal_records (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    category VARCHAR(16) NOT NULL CHECK (category IN ('5k', '10k', 'half_marathon', 'marathon', 'longest_run')),
    activity_id UUID REFERENCES activities(id) ON DELETE SET NULL,
    distance_m DOUBLE PRECISION NOT NULL,
    duration_seconds BIGINT NOT NULL,
    pace_seconds_per_km DOUBLE PRECISION NOT NULL,
    achieved_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, category)
);

CREATE INDEX idx_personal_records_user_id ON personal_records(user_id);
