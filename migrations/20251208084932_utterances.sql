-- Add migration script here
CREATE TABLE channels (
    id UUID PRIMARY KEY NOT NULL,
    recording UUID NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
    idx_in_file INTEGER NOT NULL,
    assigned_name TEXT,
    wav2vec2_age_gender JSONB NOT NULL
);

CREATE TABLE utterances (
    id UUID PRIMARY KEY NOT NULL,
    channel UUID NOT NULL REFERENCES channels(id),
    start_sec REAL NOT NULL,
    end_sec REAL NOT NULL,
    content TEXT NOT NULL,
    emotion2vec JSONB NOT NULL,
    wav2vec2_emotion JSONB NOT NULL,
    whisper JSONB NOT NULL
);

CREATE INDEX utterance_start ON utterances(channel, start_sec);
CREATE INDEX utterance_end ON utterances(channel, end_sec);