-- Add migration script here

ALTER TABLE recordings ADD COLUMN original_transcript_s3_path VARCHAR(255);
ALTER TABLE recordings ADD COLUMN force_diarize BOOLEAN NOT NULL DEFAULT false;