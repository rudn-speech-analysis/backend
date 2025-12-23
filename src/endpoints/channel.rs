use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};

use crate::{
    AppState, message_queue::types::MetricCollection, result::AppResult, url::UrlGenerator,
};

pub async fn get_channel(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    url: UrlGenerator,
) -> AppResult<Json<ChannelData>> {
    let row = sqlx::query!(
        r#"SELECT *, metrics_list as "metrics: sqlx::types::Json<Vec<MetricCollection>>" FROM channels WHERE id=$1"#,
        id
    )
    .fetch_optional(&state.db)
    .await?;

    let Some(row) = row else {
        return Err(eyre::eyre!("channel not found").into());
    };

    let segments_begin_url = url.url(format!("/channels/{}/segments?start=0&end=3600", id));

    Ok(Json(ChannelData {
        self_url: url.url(format!("/channels/{}", id)),
        recording_url: url.url(format!("/recordings/{}", row.recording)),
        idx_in_file: row.idx_in_file,
        assigned_name: row.assigned_name,
        metrics: row.metrics.0,
        segments_begin_url,
    }))
}

pub async fn set_assigned_name(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    Json(name): Json<Option<String>>,
) -> AppResult<axum::http::StatusCode> {
    tracing::info!("set assigned name for channel {id} to {name:?}");
    if let Some(name) = name {
        sqlx::query!("UPDATE channels SET assigned_name=$1 WHERE id=$2", name, id)
            .execute(&state.db)
            .await?;
        Ok(axum::http::StatusCode::NO_CONTENT)
    } else {
        sqlx::query!("UPDATE channels SET assigned_name=NULL WHERE id=$1", id)
            .execute(&state.db)
            .await?;
        return Ok(axum::http::StatusCode::NO_CONTENT);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChannelData {
    self_url: String,
    recording_url: String,
    idx_in_file: i32,
    assigned_name: Option<String>,
    segments_begin_url: String,
    metrics: Vec<MetricCollection>,
}
