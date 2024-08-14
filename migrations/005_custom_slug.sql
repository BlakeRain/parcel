ALTER TABLE uploads
  ADD COLUMN custom_slug TEXT;

CREATE UNIQUE INDEX uploads_custom_slug_uindex ON uploads (uploaded_by, custom_slug);
