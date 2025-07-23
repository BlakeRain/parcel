-- Add new columns to the 'team_members' table to store the permissions of the user in the team.
ALTER TABLE team_members ADD COLUMN can_edit BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE team_members ADD COLUMN can_delete BOOLEAN NOT NULL DEFAULT TRUE;
