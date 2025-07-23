-- Add a new column to the 'users' table to track the last access time
ALTER TABLE users ADD COLUMN last_access TIMESTAMP WITH TIME ZONE;
