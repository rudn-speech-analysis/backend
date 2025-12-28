use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    AppState, message_queue::types::MetricCollection, result::AppResult, url::UrlGenerator,
};

#[derive(serde::Serialize)]
pub struct RecordingRow {
    url: String,
    id: uuid::Uuid,
}

pub async fn list_recordings(
    url: UrlGenerator,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<RecordingRow>>> {
    let rows = sqlx::query!("SELECT id FROM recordings")
        .fetch_all(&state.db)
        .await?;

    Ok(Json(
        rows.into_iter()
            .map(|row| RecordingRow {
                id: row.id,
                url: url.url(format!("/recordings/{}", row.id)),
            })
            .collect(),
    ))
}

pub async fn get_recording(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    url: UrlGenerator,
) -> AppResult<Json<RecordingData>> {
    let row = sqlx::query!(
        "SELECT recordings.*,
    recording_stats.metrics_list AS \"metrics: Option<sqlx::types::Json<Vec<MetricCollection>>>\"
    FROM recordings
    LEFT JOIN recording_stats ON recordings.id = recording_stats.recording_id
    WHERE id=$1",
        id
    )
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
        self_url: url.url(format!("/recordings/{}", id)),
        id: row.id,
        uploaded_at: row.uploaded_at,
        download_url,
        channels: channel_urls,
        metrics: row.metrics.map(|m| m.0),
        analysis_status: row.analysis_status,
        analysis_percent_done: row.analysis_percent as f32,
        analysis_error_message: row.analysis_error,
        analysis_channel: row.analysis_channel,
        analysis_description: row.analysis_description,
        analysis_updated_at: row.analysis_last_update.unwrap_or(row.uploaded_at),
    }))
}

#[derive(serde::Serialize)]
pub struct RecordingData {
    self_url: String,
    id: uuid::Uuid,
    uploaded_at: chrono::DateTime<chrono::Utc>,
    download_url: String,
    channels: Vec<String>,
    metrics: Option<Vec<MetricCollection>>,
    analysis_status: String,
    analysis_percent_done: f32,
    analysis_channel: Option<i32>,
    analysis_description: Option<String>,
    analysis_error_message: Option<String>,
    analysis_updated_at: chrono::DateTime<chrono::Utc>,
}

pub async fn delete_recording(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> AppResult<axum::http::StatusCode> {
    sqlx::query!("DELETE FROM recordings WHERE id=$1", id)
        .execute(&state.db)
        .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
