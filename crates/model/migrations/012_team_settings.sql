-- Add new columns to the 'team_members' table to indicate the member is able to edit team settings.
ALTER TABLE team_members ADD COLUMN can_config BOOLEAN NOT NULL DEFAULT FALSE;

