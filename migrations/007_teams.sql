-- Create a new table for the teams.
CREATE TABLE teams (
  id TEXT NOT NULL PRIMARY KEY,
  name TEXT NOT NULL,
  slug TEXT NOT NULL,
  "limit" BIGINT,
  enabled BOOLEAN NOT NULL,
  created_at TIMESTAMP NOT NULL,
  created_by TEXT NOT NULL REFERENCES users (id)
);

-- Teams can be identified by their "slug", just as users and uploads.
CREATE UNIQUE INDEX teams_slug_uindex ON teams (slug);

-- Create a table that will hold the correlation between teams and users.
CREATE TABLE team_members (
  team TEXT NOT NULL REFERENCES teams (id),
  user TEXT NOT NULL REFERENCES users (id),
  PRIMARY KEY (team, user)
);

-- Add the columns for the team or user ownership of an upload.
ALTER TABLE uploads
  ADD COLUMN owner_user TEXT REFERENCES users (id);
ALTER TABLE uploads
  ADD COLUMN owner_team TEXT REFERENCES teams (id);

-- As we're adding the teams functionality, all the uploads will be owned by whoever uploaded them.
UPDATE uploads SET owner_user = uploaded_by;

-- Add a generated column that will contain either of the owner UUIDs.
ALTER TABLE uploads
  ADD COLUMN owner TEXT NOT NULL GENERATED ALWAYS AS (
    CASE
      WHEN owner_user IS NOT NULL THEN owner_user
      ELSE owner_team
    END
  ) VIRTUAL;

-- We drop the old unique index on uploads that made sure that slugs were unique, as that index was
-- against the `uploaded_by` column, which is no longer used to uniquely identity the owner.
-- Instead, we want to create an index that uses the `owner_user` and `owner_team` columns that we
-- combined into the single 'owner' column.
DROP INDEX uploads_custom_slug_uindex;
CREATE UNIQUE INDEX uploads_custom_slug_uidx ON uploads (owner, custom_slug);
