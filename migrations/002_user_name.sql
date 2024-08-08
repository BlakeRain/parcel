ALTER TABLE users
  ADD COLUMN name TEXT NOT NULL DEFAULT('');

UPDATE users
  SET name = username;
