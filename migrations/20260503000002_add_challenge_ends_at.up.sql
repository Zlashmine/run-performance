-- Add optional end date to challenges for progression window capping.
ALTER TABLE challenges
    ADD COLUMN ends_at TIMESTAMPTZ;

-- Index to speed up find_activities_by_user_from queries used by the
-- progression engine. Without this, every recalculation does a full
-- table scan for the user's activities.
CREATE INDEX IF NOT EXISTS idx_activities_user_date
    ON activities (user_id, date ASC);
