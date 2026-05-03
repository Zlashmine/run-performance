-- Activities: unique constraint prevents duplicate uploads (user_id + date)
-- Replaces the fragile application-level SELECT-before-INSERT dedup (D4)
ALTER TABLE activities
    ADD CONSTRAINT uq_activities_user_date UNIQUE (user_id, date);

-- Enforce referential integrity: activities must belong to a real user
ALTER TABLE activities
    ADD CONSTRAINT fk_activities_user
        FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE;

-- Remove the redundant PK index (primary key is already indexed by Postgres)
DROP INDEX IF EXISTS idx_activities_id;

-- Composite covering index for the most common query pattern
CREATE INDEX idx_activities_user_date ON activities (user_id, date DESC);

-- Users: prevent duplicate accounts created by concurrent sign-in attempts
ALTER TABLE users
    ADD CONSTRAINT uq_users_email     UNIQUE (email);
ALTER TABLE users
    ADD CONSTRAINT uq_users_google_id UNIQUE (google_id);
