-- Add migration script here
ALTER TABLE recordings ADD COLUMN analysis_status VARCHAR(255) NOT NULL DEFAULT 'pending';
ALTER TABLE recordings ADD COLUMN analysis_percent INTEGER NOT NULL DEFAULT 0;
ALTER TABLE recordings ADD COLUMN analysis_error TEXT DEFAULT NULL;
ALTER TABLE recordings ADD COLUMN analysis_started TIMESTAMPTZ DEFAULT NULL;
ALTER TABLE recordings ADD COLUMN analysis_last_update TIMESTAMPTZ DEFAULT NULL;
