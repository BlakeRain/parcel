-- Create a temporary table that we're going to use for the new users.
CREATE TABLE new_users AS SELECT * FROM users;

-- Create a table that maps from the old user ID to a UUID.
CREATE TABLE new_user_uuids (
  uuid TEXT NOT NULL,
  id INTEGER NOT NULL
);

INSERT INTO new_user_uuids (id, uuid)
  SELECT new_users.id as id,
    lower(hex(randomblob(4))) || '-' ||
    lower(hex(randomblob(2))) || '-' ||
    '4' ||
    substr(lower(hex(randomblob(2))), 2) || '-' ||
    substr('89ab', abs(random()) % 4 + 1, 1) ||
    substr(lower(hex(randomblob(2))), 2) || '-' ||
    lower(hex(randomblob(6))) as uuid
  FROM new_users;

-- Now create a temporary table for the uploads.
CREATE TABLE new_uploads AS SELECT * FROM uploads;

-- As before, create a table that maps from the old upload ID to a UUID.
CREATE TABLE new_upload_uuids (
  uuid TEXT NOT NULL,
  id INTEGER NOT NULL
);

INSERT INTO new_upload_uuids (id, uuid)
  SELECT new_uploads.id as id,
    lower(hex(randomblob(4))) || '-' ||
    lower(hex(randomblob(2))) || '-' ||
    '4' ||
    substr(lower(hex(randomblob(2))), 2) || '-' ||
    substr('89ab', abs(random()) % 4 + 1, 1) ||
    substr(lower(hex(randomblob(2))), 2) || '-' ||
    lower(hex(randomblob(6))) as uuid
  FROM new_uploads;

-- Now we can drop the old uploads and users tables.
DROP INDEX uploads_slug_uindex;
DROP TABLE uploads;
DROP INDEX users_username_uindex;
DROP TABLE users;

-- Now we can create a new table for the users.
CREATE TABLE users (
  id TEXT NOT NULL PRIMARY KEY,
  username TEXT NOT NULL,
  name TEXT NOT NULL,
  password TEXT NOT NULL,
  totp TEXT,
  enabled BOOLEAN NOT NULL,
  admin BOOLEAN NOT NULL,
  "limit" BIGINT,
  created_at TIMESTAMP NOT NULL,
  created_by TEXT
);

CREATE INDEX users_username_uindex ON users (username);

-- Populate the new users table, substituting the UUID for each user.
INSERT INTO users (
  id,
  username,
  name,
  password,
  totp,
  enabled,
  admin,
  "limit",
  created_at,
  created_by
  ) SELECT
  new_user_uuids.uuid as id,
  new_users.username,
  new_users.name,
  new_users.password,
  new_users.totp,
  new_users.enabled,
  new_users.admin,
  new_users."limit",
  new_users.created_at,
  creator.id as created_by
  FROM new_users
  JOIN new_user_uuids ON new_users.id = new_user_uuids.id
  JOIN new_user_uuids AS creator ON new_users.created_by = creator.id;

-- Now we can create the new uploads table.
CREATE TABLE uploads (
  id TEXT NOT NULL PRIMARY KEY,
  slug TEXT NOT NULL,
  filename TEXT NOT NULL,
  size BIGINT NOT NULL,
  public BOOLEAN NOT NULL,
  downloads BIGINT NOT NULL,
  "limit" BIGINT,
  remaining BIGINT,
  expiry_date DATE,
  password TEXT,
  custom_slug TEXT,
  uploaded_by TEXT NOT NULL REFERENCES users(id),
  uploaded_at DATETIME NOT NULL,
  remote_addr TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS uploads_slug_uindex ON uploads (slug);
CREATE UNIQUE INDEX uploads_custom_slug_uindex ON uploads (uploaded_by, custom_slug);

-- Populate the new uploads table, substituting the UUID for each upload.
INSERT INTO uploads (
  id,
  slug,
  filename,
  size,
  public,
  downloads,
  "limit",
  remaining,
  expiry_date,
  password,
  custom_slug,
  uploaded_by,
  uploaded_at,
  remote_addr
) SELECT
    new_upload_uuids.uuid as id,
    new_uploads.slug,
    new_uploads.filename,
    new_uploads.size,
    new_uploads.public,
    new_uploads.downloads,
    new_uploads."limit",
    new_uploads.remaining,
    new_uploads.expiry_date,
    new_uploads.password,
    new_uploads.custom_slug,
    new_user_uuids.uuid as uploaded_by,
    new_uploads.uploaded_at,
    new_uploads.remote_addr
    FROM new_uploads
    JOIN new_user_uuids ON new_uploads.uploaded_by = new_user_uuids.id
    JOIN new_upload_uuids ON new_uploads.id = new_upload_uuids.id;

-- Clean up the temporary tables.
DROP TABLE new_users;
DROP TABLE new_user_uuids;
DROP TABLE new_uploads;
DROP TABLE new_upload_uuids;
