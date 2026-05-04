-- Restore the CHECK constraint if rolling back.
ALTER TABLE challenge_workout_requirements
    ADD CONSTRAINT requirement_type_check CHECK (requirement_type IN (
        'pace_faster_than',
        'distance_longer_than',
        'days_since_challenge_start',
        'days_since_first_workout',
        'faster_than_previous'
    ));
