CREATE TABLE
    trackpoints (
        id UUID PRIMARY KEY,
        lat TEXT NOT NULL,
        lon TEXT NOT NULL,
        elevation REAL NOT NULL,
        time TEXT NOT NULL,
        activity_id UUID NOT NULL REFERENCES activities (id) ON DELETE CASCADE
    );

CREATE INDEX idx_trackpoints_activity_id ON trackpoints (activity_id);