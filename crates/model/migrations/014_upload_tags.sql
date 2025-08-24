-- Adds tags to uploads. Tags are just short strings, and uploads can have multiple tags.

CREATE TABLE tags (
  id TEXT NOT NULL PRIMARY KEY,
  name TEXT NOT NULL,
  user TEXT,
  team TEXT,
  UNIQUE (name, user),
  UNIQUE (name, team),
  CHECK ((user IS NOT NULL AND team IS NULL) OR (user IS NULL AND team IS NOT NULL)),
  FOREIGN KEY (user) REFERENCES users (id) ON DELETE CASCADE,
  FOREIGN KEY (team) REFERENCES teams (id) ON DELETE CASCADE
);

CREATE TABLE upload_tags (
  upload TEXT NOT NULL REFERENCES uploads (id) ON DELETE CASCADE,
  tag TEXT NOT NULL REFERENCES tags (id) ON DELETE CASCADE,
  PRIMARY KEY (upload, tag)
);
