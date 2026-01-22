-- Add index on team_members.user for efficient user-based lookups.
-- The primary key is (team, user), so lookups by user alone require a full scan.
-- This index improves performance for Team::get_for_user, has_teams, get_teams queries.
CREATE INDEX team_members_user_idx ON team_members (user);
