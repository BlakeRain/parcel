-- Add a new column to users that stores their default sort preferences.
ALTER TABLE users ADD COLUMN default_order TEXT NOT NULL DEFAULT 'uploaded_at';
ALTER TABLE users ADD COLUMN default_asc BOOLEAN NOT NULL DEFAULT FALSE;
