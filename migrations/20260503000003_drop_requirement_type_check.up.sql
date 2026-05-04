-- Drop the hand-maintained CHECK constraint on requirement_type.
-- Validation is now enforced by the RequirementType Rust enum (serde
-- rejects unknown variants at deserialization time), so the constraint
-- is redundant.
ALTER TABLE challenge_workout_requirements
    DROP CONSTRAINT IF EXISTS requirement_type_check;
