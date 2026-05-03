-- Add a unique constraint on (activity_id, time) so ON CONFLICT DO NOTHING
-- works reliably for trackpoint deduplication (Q17 fix).
--
-- Note: after migration 20250522000001 the `time` column is TIMESTAMPTZ,
-- so this composite uniqueness is both correct and efficient.
ALTER TABLE trackpoints
    ADD CONSTRAINT uq_trackpoints_activity_time UNIQUE (activity_id, time);
