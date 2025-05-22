CREATE TABLE activities (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    date TIMESTAMP NOT NULL,
    name TEXT NOT NULL,
    activity_type TEXT NOT NULL,
    distance REAL NOT NULL,
    duration TEXT NOT NULL,
    average_pace REAL NOT NULL,
    average_speed REAL NOT NULL,
    calories REAL NOT NULL,
    climb REAL NOT NULL,
    gps_file TEXT NOT NULL
);
CREATE INDEX idx_activities_id ON activities (id);
CREATE INDEX idx_activities_user_id ON activities (user_id);
CREATE INDEX idx_activities_type ON activities (activity_type);