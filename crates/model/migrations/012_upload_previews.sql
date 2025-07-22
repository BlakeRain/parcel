-- This migration script adds preview information to the uploads table.
ALTER TABLE uploads ADD COLUMN mime_type TEXT;
ALTER TABLE uploads ADD COLUMN has_preview BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE uploads ADD COLUMN preview_error TEXT;
