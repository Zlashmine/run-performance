-- Migration 20260503000003 tried to drop the CHECK constraint on
-- requirement_type using the name 'requirement_type_check', but PostgreSQL
-- auto-names inline CHECK constraints as '{table}_{column}_check'.
-- The actual constraint name is 'challenge_workout_requirements_requirement_type_check'.
-- The IF EXISTS clause caused the previous migration to silently do nothing.
ALTER TABLE challenge_workout_requirements
    DROP CONSTRAINT IF EXISTS challenge_workout_requirements_requirement_type_check;
