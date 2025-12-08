use serde::{Deserialize, Serialize};
use sqlx::types::Json;

use crate::AppState;

pub async fn analyze_recording(state: AppState, rec_id: uuid::Uuid) -> eyre::Result<()> {
    let row = sqlx::query!("SELECT * FROM recordings WHERE id=$1", rec_id)
        .fetch_one(&state.db)
        .await?;

    let audio_url = state
        .s3
        .presign_get(row.original_s3_path, 3600, None)
        .await?;

    // TODO: submit audio_url to analysis queue
    let outcome: SingleChannelAnalysisOutcome =
        serde_json::from_str(include_str!("sample_metrics.json"))?;

    let mut tx = state.db.begin().await?;
    let channels = vec![outcome];
    for (i, channel) in channels.iter().enumerate() {
        let i = i as i32;
        let id = uuid::Uuid::new_v4();

        sqlx::query!(
            "INSERT INTO channels (id, recording, idx_in_file, wav2vec2_age_gender) VALUES ($1, $2, $3, $4)",
            id,
            rec_id,
            i,
            Json(&channel.recording.wav2vec2_age_gender) as _
        )
        .execute(&mut *tx)
        .await?;

        for phrase in &channel.phrase {
            sqlx::query!(
                "INSERT INTO utterances (id, channel, start_sec, end_sec, content, emotion2vec, wav2vec2_emotion, whisper) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                uuid::Uuid::new_v4(),
                id,
                phrase.whisper.start,
                phrase.whisper.end,
                phrase.whisper.text,
                Json(&phrase.emotion2vec) as _,
                Json(&phrase.wav2vec2_emotion) as _,
                Json(&phrase.whisper) as _
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SingleChannelAnalysisOutcome {
    recording: RecordingStats,
    phrase: Vec<PhraseStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecordingStats {
    wav2vec2_age_gender: Wav2Vec2AgeGender,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
pub struct Wav2Vec2AgeGender {
    age: f32,
    female: f32,
    male: f32,
    child: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhraseStats {
    pub emotion2vec: Emotion2Vec,
    pub wav2vec2_emotion: Wav2Vec2Emotion,
    pub whisper: Whisper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Emotion2Vec {
    angry: f32,
    disgusted: f32,
    fearful: f32,
    happy: f32,
    neutral: f32,
    other: f32,
    sad: f32,
    surprised: f32,
    #[serde(rename = "<unk>")]
    unk: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wav2Vec2Emotion {
    arousal: f32,
    dominance: f32,
    valence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Whisper {
    id: u64,
    seek: u64,
    start: f32,
    end: f32,
    text: String,
    tokens: Vec<u64>,
    temperature: f32,
    avg_logprob: f32,
    compression_ratio: f32,
    no_speech_prob: f32,
    words: Vec<WhisperWord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhisperWord {
    word: String,
    start: f32,
    end: f32,
    probability: f32,
}
