-- Add migration script here
CREATE TABLE channels (
    id UUID PRIMARY KEY NOT NULL,
    recording UUID NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
    idx_in_file INTEGER NOT NULL,
    assigned_name TEXT,
    wav2vec2_age_gender JSONB NOT NULL
);

CREATE TABLE segments (
    id UUID PRIMARY KEY NOT NULL,
    channel UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    start_sec REAL NOT NULL,
    end_sec REAL NOT NULL,
    content TEXT NOT NULL,
    wav2vec2_emotion JSONB NOT NULL,
    whisper JSONB NOT NULL
);

CREATE INDEX segment_start ON segments(channel, start_sec);
CREATE INDEX segment_end ON segments(channel, end_sec);