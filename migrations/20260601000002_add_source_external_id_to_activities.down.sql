DROP INDEX IF EXISTS uq_activities_external_source;
ALTER TABLE activities DROP COLUMN IF EXISTS external_id;
ALTER TABLE activities DROP COLUMN IF EXISTS source;
