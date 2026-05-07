CREATE TABLE strava_tokens (
    -- One row per user; the user must exist in the users table.
    user_id             UUID        PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,

    -- Strava's numeric athlete ID — used to look up the user from webhook events.
    strava_athlete_id   BIGINT      NOT NULL,
    strava_athlete_name TEXT        NOT NULL DEFAULT '',

    -- OAuth tokens. access_token expires at expires_at (Unix epoch seconds).
    access_token        TEXT        NOT NULL,
    refresh_token       TEXT        NOT NULL,
    expires_at          BIGINT      NOT NULL,  -- Unix epoch seconds

    -- Incremental sync checkpoint: next sync fetches activities after this timestamp.
    last_synced_at      TIMESTAMPTZ,

    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for the webhook lookup path: strava_athlete_id → user_id.
CREATE INDEX idx_strava_tokens_athlete_id ON strava_tokens (strava_athlete_id);
