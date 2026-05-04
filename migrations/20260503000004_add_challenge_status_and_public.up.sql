-- Add lifecycle status to challenges.
-- All existing challenges default to 'draft' since they were never explicitly activated.
ALTER TABLE challenges
    ADD COLUMN status TEXT NOT NULL DEFAULT 'draft'
        CHECK (status IN ('draft', 'pending_activation', 'active', 'expired'));

-- Public-discovery flag. Only true for ACTIVE/PENDING_ACTIVATION challenges (enforced in app).
ALTER TABLE challenges
    ADD COLUMN is_public BOOLEAN NOT NULL DEFAULT false;

-- Nullable FK back to the challenge this was cloned from.
-- ON DELETE SET NULL: if the original is deleted, clones lose the reference but survive.
ALTER TABLE challenges
    ADD COLUMN parent_challenge_id UUID
        REFERENCES challenges (id) ON DELETE SET NULL;

-- Index for the discover feed: finds public challenges efficiently.
CREATE INDEX idx_challenges_public
    ON challenges (is_public, status, created_at DESC)
    WHERE is_public = true;

-- Index for participant count + list queries.
CREATE INDEX idx_challenges_parent_id
    ON challenges (parent_challenge_id)
    WHERE parent_challenge_id IS NOT NULL;

-- Index to speed up find_active_challenges_for_user after status column is added.
CREATE INDEX idx_challenges_user_active
    ON challenges (user_id, started_at)
    WHERE status = 'active';

-- ─── Data migration for pre-existing rows ────────────────────────────────────
-- All rows received status='draft' via DEFAULT above.
-- Back-fill to the correct status based on their existing start/end dates so
-- the progression engine continues to work immediately after the migration.
-- Challenges without started_at stay as 'draft' (already the default).

-- Future start date → waiting to begin.
UPDATE challenges
    SET status = 'pending_activation'
WHERE started_at IS NOT NULL
  AND started_at > NOW();

-- Past start date, not yet ended (or no end date) → currently running.
UPDATE challenges
    SET status = 'active'
WHERE started_at IS NOT NULL
  AND started_at <= NOW()
  AND (ends_at IS NULL OR ends_at > NOW());

-- Past end date → finished.
UPDATE challenges
    SET status = 'expired'
WHERE started_at IS NOT NULL
  AND ends_at IS NOT NULL
  AND ends_at <= NOW();
