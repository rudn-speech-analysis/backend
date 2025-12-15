-- Add migration script here
CREATE TABLE recording_stats (
    recording_id UUID NOT NULL PRIMARY KEY REFERENCES recordings(id) ON DELETE CASCADE,
    duration_seconds REAL NOT NULL,
    channels INTEGER NOT NULL,
    bit_depth INTEGER NOT NULL,
    sample_rate INTEGER NOT NULL,
    max_dbfs REAL NOT NULL,
    rms REAL NOT NULL,
    raw_data_length INTEGER NOT NULL
);