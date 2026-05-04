-- Speed up participant-count joins on the public challenges list query.
-- challenges.parent_challenge_id has no index by default; this prevents a
-- sequential scan when LEFT JOINing clones ON clones.parent_challenge_id = c.id.
CREATE INDEX IF NOT EXISTS idx_challenges_parent_id ON challenges (parent_challenge_id);
