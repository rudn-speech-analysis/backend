-- Add migration script here
CREATE TABLE recordings (
    id UUID PRIMARY KEY NOT NULL,
    uploaded_at TIMESTAMP NOT NULL,
    original_filename VARCHAR(255) NOT NULL,
    original_s3_path VARCHAR(255) NOT NULL
)