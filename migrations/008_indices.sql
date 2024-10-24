-- As the model is getting a bit more use, time to add some indices to speed up queries.

-- Create indices for the uploads on each of the sort columns that we use, first where the owner is
-- the user and then where the owner is the team.

CREATE INDEX uploads_user_filename_idx    ON uploads (owner_user, filename);
CREATE INDEX uploads_user_size_idx        ON uploads (owner_user, size);
CREATE INDEX uploads_user_downloads_idx   ON uploads (owner_user, downloads);
CREATE INDEX uploads_user_expiry_date_idx ON uploads (owner_user, expiry_date);
CREATE INDEX uploads_user_uploaded_at_idx ON uploads (owner_user, uploaded_at);

CREATE INDEX uploads_team_filename_idx    ON uploads (owner_team, filename);
CREATE INDEX uploads_team_size_idx        ON uploads (owner_team, size);
CREATE INDEX uploads_team_downloads_idx   ON uploads (owner_team, downloads);
CREATE INDEX uploads_team_expiry_date_idx ON uploads (owner_team, expiry_date);
CREATE INDEX uploads_team_uploaded_at_idx ON uploads (owner_team, uploaded_at);
