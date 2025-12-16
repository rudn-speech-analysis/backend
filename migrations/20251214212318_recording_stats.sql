-- Add migration script here
CREATE TABLE recording_stats (
    recording_id UUID NOT NULL PRIMARY KEY REFERENCES recordings(id) ON DELETE CASCADE,
    metrics_list JSONB NOT NULL
);