use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};

use crate::{AppState, analysis_submit::Wav2Vec2AgeGender, result::AppResult, url::UrlGenerator};

pub async fn get_channel(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    url: UrlGenerator,
) -> AppResult<Json<ChannelData>> {
    let row = sqlx::query!(
        r#"SELECT *, wav2vec2_age_gender as "w2v: sqlx::types::Json<Wav2Vec2AgeGender>" FROM channels WHERE id=$1"#,
        id
    )
    .fetch_optional(&state.db)
    .await?;

    let Some(row) = row else {
        return Err(eyre::eyre!("channel not found").into());
    };

    let utterances_begin_url = url.url(format!("/channels/{}/utterances?start=0&end=30", id));

    Ok(Json(ChannelData {
        recording_url: url.url(format!("/recordings/{}", row.recording)),
        idx_in_file: row.idx_in_file,
        assigned_name: row.assigned_name,
        wav2vec2_age_gender: row.w2v.0,
        utterances_begin_url,
    }))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChannelData {
    recording_url: String,
    idx_in_file: i32,
    assigned_name: Option<String>,
    wav2vec2_age_gender: Wav2Vec2AgeGender,
    utterances_begin_url: String,
}
