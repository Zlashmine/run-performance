ALTER TABLE users      DROP CONSTRAINT IF EXISTS uq_users_google_id;
ALTER TABLE users      DROP CONSTRAINT IF EXISTS uq_users_email;
DROP INDEX  IF EXISTS idx_activities_user_date;
ALTER TABLE activities DROP CONSTRAINT IF EXISTS fk_activities_user;
ALTER TABLE activities DROP CONSTRAINT IF EXISTS uq_activities_user_date;
CREATE INDEX idx_activities_id ON activities (id);
