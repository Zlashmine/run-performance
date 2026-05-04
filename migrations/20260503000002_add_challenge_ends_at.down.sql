DROP INDEX IF EXISTS idx_activities_user_date;

ALTER TABLE challenges DROP COLUMN IF EXISTS ends_at;
