use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, State},
};

use crate::{AppState, result::AppResult, url::UrlGenerator};

pub async fn get_recording(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    url: UrlGenerator,
) -> AppResult<Json<RecordingData>> {
    let row = sqlx::query!("SELECT * FROM recordings WHERE id=$1", id)
        .fetch_optional(&state.db)
        .await?;

    let Some(row) = row else {
        return Err(eyre::eyre!("recording not found").into());
    };

    let mut custom_queries = HashMap::new();
    custom_queries.insert(
        "response-content-disposition".into(),
        format!("attachment; filename=\"{}\"", row.original_filename),
    );
    let download_url = state
        .s3
        .presign_get(
            format!("original_upload/{}", row.id),
            3600,
            Some(custom_queries),
        )
        .await?;

    let channels = sqlx::query!("SELECT id FROM channels WHERE recording=$1", id)
        .fetch_all(&state.db)
        .await?;
    let channel_urls = channels
        .iter()
        .map(|c| url.url(format!("/channels/{}", c.id)))
        .collect();

    Ok(axum::Json(RecordingData {
        id: row.id,
        uploaded_at: row.uploaded_at,
        download_url,
        channels: channel_urls,
    }))
}

#[derive(serde::Serialize)]
pub struct RecordingData {
    id: uuid::Uuid,
    uploaded_at: chrono::DateTime<chrono::Utc>,
    download_url: String,
    channels: Vec<String>,
}
