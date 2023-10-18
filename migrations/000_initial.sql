CREATE TABLE IF NOT EXISTS users (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  username TEXT NOT NULL,
  password TEXT NOT NULL,
  enabled BOOLEAN NOT NULL,
  admin BOOLEAN NOT NULL,
  "limit" BIGINT,
  created_at DATETIME NOT NULL,
  created_by INTEGER
);

CREATE UNIQUE INDEX IF NOT EXISTS users_username_uindex ON users (username);

CREATE TABLE IF NOT EXISTS uploads (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  slug TEXT NOT NULL,
  filename TEXT NOT NULL,
  size BIGINT NOT NULL,
  public BOOLEAN NOT NULL,
  downloads BIGINT NOT NULL,
  "limit" BIGINT,
  expiry_date DATE,
  uploaded_by INTEGER NOT NULL REFERENCES users (id),
  uploaded_at DATETIME NOT NULL,
  remote_addr TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS uploads_slug_uindex ON uploads (slug);
