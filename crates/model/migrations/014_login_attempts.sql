-- Create a table to track login attempts for brute force protection.
CREATE TABLE login_attempts (
    id TEXT NOT NULL PRIMARY KEY,
    username TEXT NOT NULL,
    ip_address TEXT,
    attempted_at TIMESTAMP NOT NULL,
    success INTEGER NOT NULL DEFAULT 0
);

-- Index for checking recent failed attempts by username (lockout check).
CREATE INDEX login_attempts_username_attempted_at_idx
    ON login_attempts (username, attempted_at);
