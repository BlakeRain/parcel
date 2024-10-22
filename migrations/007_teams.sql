CREATE TABLE teams (
  id TEXT NOT NULL PRIMARY KEY,
  name TEXT NOT NULL,
  "limit" BIGINT,
  enabled BOOLEAN NOT NULL,
  created_at TIMESTAMP NOT NULL,
  created_by TEXT NOT NULL REFERENCES users (id)
);

CREATE TABLE team_members (
  team TEXT NOT NULL REFERENCES teams (id),
  user TEXT NOT NULL REFERENCES users (id),
  PRIMARY KEY (team, user)
);

ALTER TABLE uploads
  ADD COLUMN owner_user TEXT REFERENCES users (id);
ALTER TABLE uploads
  ADD COLUMN owner_team TEXT REFERENCES teams (id);

UPDATE uploads SET owner_user = uploaded_by;
