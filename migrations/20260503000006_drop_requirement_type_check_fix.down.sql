-- Restore the original CHECK constraint (re-adds only the 5 original types).
-- If you need to roll back further than this point you will also need to
-- delete any rows using the newer requirement types first.
ALTER TABLE challenge_workout_requirements
    ADD CONSTRAINT challenge_workout_requirements_requirement_type_check
        CHECK (requirement_type IN (
            'pace_faster_than',
            'distance_longer_than',
            'days_since_challenge_start',
            'days_since_first_workout',
            'faster_than_previous'
        ));
