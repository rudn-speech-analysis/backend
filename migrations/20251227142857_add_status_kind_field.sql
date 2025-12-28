-- Add migration script here
ALTER TABLE recordings ADD COLUMN analysis_description VARCHAR(255);
ALTER TABLE recordings ADD COLUMN analysis_channel INTEGER;