CREATE TABLE users (
    id UUID PRIMARY KEY,
    google_id TEXT NOT NULL,
    email TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_users_id ON users (id);
CREATE INDEX idx_users_google_id ON users (google_id);