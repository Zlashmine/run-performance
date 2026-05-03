-- Fix trackpoints: lat/lon stored as TEXT → DOUBLE PRECISION for real geo values
-- Fix time stored as TEXT → TIMESTAMPTZ for temporal queries
ALTER TABLE trackpoints
    ALTER COLUMN lat  TYPE DOUBLE PRECISION USING lat::DOUBLE PRECISION,
    ALTER COLUMN lon  TYPE DOUBLE PRECISION USING lon::DOUBLE PRECISION,
    ALTER COLUMN time TYPE TIMESTAMPTZ      USING time::TIMESTAMPTZ;
