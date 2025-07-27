-- Add new columns to the 'team_members' table to indicate the member is able to edit team settings.
ALTER TABLE team_members ADD COLUMN can_config BOOLEAN NOT NULL DEFAULT FALSE;

-- Drop the `NOT NULL` constraint on the `created_by` column in the `teams` table.
ALTER TABLE teams ADD COLUMN new_created_by TEXT REFERENCES users (id);
UPDATE teams SET new_created_by = created_by;
ALTER TABLE teams DROP COLUMN created_by;
ALTER TABLE teams RENAME COLUMN new_created_by TO created_by;

-- Change the 'uploaded_by' column in the 'uploads' table to allow NULL values.
ALTER TABLE uploads ADD COLUMN new_uploaded_by TEXT REFERENCES users (id);
UPDATE uploads SET new_uploaded_by = uploaded_by;
ALTER TABLE uploads DROP COLUMN uploaded_by;
ALTER TABLE uploads RENAME COLUMN new_uploaded_by TO uploaded_by;
