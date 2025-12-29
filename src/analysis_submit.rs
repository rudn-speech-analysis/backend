use crate::{
    AppState,
    message_queue::types::{AnalysisRequestInner, KafkaEnvelope},
};

pub async fn analyze_recording(state: AppState, rec_id: uuid::Uuid) -> eyre::Result<()> {
    let row = sqlx::query!("SELECT * FROM recordings WHERE id=$1", rec_id)
        .fetch_one(&state.db)
        .await?;

    let download_url = state
        .s3
        .presign_get(row.original_s3_path, 3600, None)
        .await?;
    let transcript_url = match row.original_transcript_s3_path {
        Some(path) => Some(state.s3.presign_get(path, 3600, None).await?),
        None => None,
    };

    let force_diarize = row.force_diarize;

    state
        .kafka
        .send_request(&KafkaEnvelope {
            id: rec_id,
            data: AnalysisRequestInner {
                download_url,
                transcript_url,
                force_diarize,
            },
        })
        .await?;

    // let outcome: SingleChannelAnalysisOutcome =
    //     serde_json::from_str(include_str!("sample_metrics.json"))?;

    // let mut tx = state.db.begin().await?;
    // let channels = vec![outcome];
    // for (i, channel) in channels.iter().enumerate() {
    //     let i = i as i32;
    //     let id = uuid::Uuid::new_v4();

    //     sqlx::query!(
    //         "INSERT INTO channels (id, recording, idx_in_file, wav2vec2_age_gender) VALUES ($1, $2, $3, $4)",
    //         id,
    //         rec_id,
    //         i,
    //         Json(&channel.recording.wav2vec2_age_gender) as _
    //     )
    //     .execute(&mut *tx)
    //     .await?;

    //     for phrase in &channel.phrase {
    //         sqlx::query!(
    //             "INSERT INTO utterances (id, channel, start_sec, end_sec, content, emotion2vec, wav2vec2_emotion, whisper) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    //             uuid::Uuid::new_v4(),
    //             id,
    //             phrase.whisper.start,
    //             phrase.whisper.end,
    //             phrase.whisper.text,
    //             Json(&phrase.emotion2vec) as _,
    //             Json(&phrase.wav2vec2_emotion) as _,
    //             Json(&phrase.whisper) as _
    //         )
    //         .execute(&mut *tx)
    //         .await?;
    //     }
    // }

    // tx.commit().await?;

    Ok(())
}
