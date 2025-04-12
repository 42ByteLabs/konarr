-- This migration will update the schema

ALTER TABLE Snapshot ADD COLUMN updated_at TEXT;

ALTER TABLE Snapshot ADD COLUMN sbom BLOB;

ALTER TABLE Snapshot ADD COLUMN error TEXT;

