DROP INDEX IF EXISTS idx_challenges_user_active;
DROP INDEX IF EXISTS idx_challenges_parent_id;
DROP INDEX IF EXISTS idx_challenges_public;
ALTER TABLE challenges DROP COLUMN parent_challenge_id;
ALTER TABLE challenges DROP COLUMN is_public;
ALTER TABLE challenges DROP COLUMN status;
