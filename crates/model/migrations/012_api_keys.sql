-- Add API keys table and aassociations to users and teams.

CREATE TABLE api_keys (
  id TEXT NOT NULL PRIMARY KEY,
  owner TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  code TEXT NOT NULL,
  name TEXT NOT NULL,
  enabled BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMP NOT NULL,
  created_by TEXT REFERENCES users(id) ON DELETE SET NULL,
  last_used TIMESTAMP
);

-- Make sure that the code for an API key is unique.
CREATE UNIQUE INDEX api_keys_code_uindex ON api_keys (code);
