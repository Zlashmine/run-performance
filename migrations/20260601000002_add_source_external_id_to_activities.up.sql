ALTER TABLE activities
    ADD COLUMN source      VARCHAR(20) NOT NULL DEFAULT 'runkeeper',
    ADD COLUMN external_id VARCHAR(64);

-- Partial unique index for deduplication: only applies when external_id is set.
-- Runkeeper rows (external_id IS NULL) are excluded from this index, preserving
-- the existing ON CONFLICT (user_id, date) behaviour for Runkeeper imports.
CREATE UNIQUE INDEX uq_activities_external_source
    ON activities (user_id, source, external_id)
    WHERE external_id IS NOT NULL;
