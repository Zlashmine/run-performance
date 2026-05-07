-- Composite index on activities for heatmap WHERE filters (user_id, activity_type, date)
CREATE INDEX IF NOT EXISTS idx_activities_user_type_date
    ON activities (user_id, activity_type, date DESC);
