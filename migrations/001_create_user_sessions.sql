-- Create user_sessions table for session management
-- This matches the schema described in the README.md

CREATE TABLE IF NOT EXISTS user_sessions (
    id VARCHAR(36) PRIMARY KEY,  -- UUID v4 as string
    username VARCHAR(255) NOT NULL,  -- Auto-generated pet name
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),  -- Session creation time
    expires_at TIMESTAMPTZ NOT NULL  -- Session expiration time
);

-- Index for efficient cleanup of expired sessions
CREATE INDEX IF NOT EXISTS idx_user_sessions_expires_at ON user_sessions(expires_at);

-- Index for efficient username lookups (if needed)
CREATE INDEX IF NOT EXISTS idx_user_sessions_username ON user_sessions(username); 