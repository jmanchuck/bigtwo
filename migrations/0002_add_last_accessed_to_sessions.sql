-- Add migration script here

-- Add last_accessed column to track when sessions were last used
ALTER TABLE user_sessions 
ADD COLUMN last_accessed TIMESTAMPTZ DEFAULT NOW();
