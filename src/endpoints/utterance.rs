use axum::{
    Json,
    extract::{Path, Query, State},
};

use crate::{AppState, analysis_submit::PhraseStats, result::AppResult, url::UrlGenerator};

#[derive(Debug, serde::Deserialize)]
pub struct GetUtterancesQuery {
    start: Option<f32>,
    end: Option<f32>,
}

pub async fn get_utterances(
    State(state): State<AppState>,
    Path(channel_id): Path<uuid::Uuid>,
    Query(query): Query<GetUtterancesQuery>,
    url: UrlGenerator,
) -> AppResult<Json<GetUtterancesResponse>> {
    let channel = sqlx::query!("SELECT * FROM channels WHERE id=$1", channel_id)
        .fetch_optional(&state.db)
        .await?;

    let Some(channel) = channel else {
        return Err(eyre::eyre!("channel not found").into());
    };

    use sqlx::types::Json as SJson;
    let utterances_inside_bounds = sqlx::query!(
        r#"
        SELECT *,
        emotion2vec as "e2v: SJson<crate::analysis_submit::Emotion2Vec>",
        wav2vec2_emotion as "w2v_e: SJson<crate::analysis_submit::Wav2Vec2Emotion>",
        whisper as "w: SJson<crate::analysis_submit::Whisper>"
        FROM utterances
        WHERE channel=$1
        AND start_sec >= $2
        AND end_sec <= $3
        ORDER BY start_sec
        "#,
        channel_id,
        query.start.unwrap_or(0.0),
        query.end.unwrap_or(f32::MAX)
    )
    .fetch_all(&state.db)
    .await?;

    let utterances = utterances_inside_bounds
        .into_iter()
        .map(|row| SingleUtteranceResponse {
            start: row.start_sec,
            end: row.end_sec,
            text: row.content.clone(),
            stats: PhraseStats {
                emotion2vec: row.e2v.0,
                wav2vec2_emotion: row.w2v_e.0,
                whisper: row.w.0,
            },
        })
        .collect();

    let mut next_url = None;
    let mut prev_url = None;

    let prev_utterance = sqlx::query!(
        r#"
        SELECT start_sec, end_sec FROM utterances
        WHERE channel=$1
        AND end_sec < $2
        LIMIT 1
        "#,
        channel_id,
        query.start.unwrap_or(0.0)
    )
    .fetch_optional(&state.db)
    .await?;

    if let Some(utter) = prev_utterance {
        let end_time = utter.end_sec;
        let start_time = (utter.start_sec - 30.0).min(0.0);
        prev_url = Some(url.url(format!(
            "/channels/{channel_id}/utterances?start={start_time}&end={end_time}",
        )));
    }

    let next_utterance = sqlx::query!(
        r#"
        SELECT start_sec, end_sec FROM utterances
        WHERE channel=$1
        AND start_sec > $2
        LIMIT 1
        "#,
        channel_id,
        query.end.unwrap_or(f32::MAX)
    )
    .fetch_optional(&state.db)
    .await?;

    if let Some(utter) = next_utterance {
        let start_time = utter.start_sec;
        let end_time = utter.end_sec + 30.0;
        next_url = Some(url.url(format!(
            "/channel/{channel_id}/utterances?start={start_time}&end={end_time}",
        )));
    }

    Ok(Json(GetUtterancesResponse {
        prev_url,
        next_url,
        utterances,
    }))
}

#[derive(Debug, serde::Serialize)]
pub struct GetUtterancesResponse {
    pub prev_url: Option<String>,
    pub next_url: Option<String>,
    pub utterances: Vec<SingleUtteranceResponse>,
}

#[derive(Debug, serde::Serialize)]
pub struct SingleUtteranceResponse {
    pub start: f32,
    pub end: f32,
    pub text: String,
    pub stats: PhraseStats,
}
