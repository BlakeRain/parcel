CREATE TABLE IF NOT EXISTS users (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  username TEXT NOT NULL,
  password TEXT NOT NULL,
  enabled BOOLEAN NOT NULL,
  admin BOOLEAN NOT NULL,
  created_at DATETIME NOT NULL,
  created_by INTEGER
);

CREATE UNIQUE INDEX IF NOT EXISTS users_username_uindex ON users (username);

CREATE TABLE IF NOT EXISTS uploads (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  slug TEXT NOT NULL,
  filename TEXT NOT NULL,
  public BOOLEAN NOT NULL,
  encrypted BOOLEAN NOT NULL,
  downloads INTEGER NOT NULL,
  remaining INTEGER,
  expiry_date DATETIME,
  uploaded_by INTEGER NOT NULL REFERENCES users (id),
  uploaded_at DATETIME NOT NULL,
  remote_addr TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS uploads_slug_uindex ON uploads (slug);
