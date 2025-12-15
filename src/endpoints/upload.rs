use axum::{
    Json,
    extract::{Multipart, State},
    http::{HeaderMap, StatusCode},
};
use eyre::{Context, OptionExt, eyre};
use futures_util::TryStreamExt;

use crate::{AppState, analysis_submit::analyze_recording, result::AppResult};

pub async fn upload_audio_file(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, HeaderMap, Json<UploadResponse>)> {
    let uuid = uuid::Uuid::new_v4();
    let field = multipart.next_field().await?;
    let Some(field) = field else {
        return Err(eyre!("no file in upload").into());
    };

    let filename = field
        .file_name()
        .ok_or_eyre("uploaded file should have filename")?
        .to_owned();
    // let content_type = field
    //     .content_type()
    //     .ok_or_eyre("uploaded file should have content type")?
    //     .to_owned();
    // if !content_type.starts_with("audio/") {
    //     return Err(eyre!("file is not audio").into());
    // }

    let mut reader = tokio_util::io::StreamReader::new(field.map_err(|multipart_error| {
        std::io::Error::new(std::io::ErrorKind::Other, multipart_error)
    }));

    let response = state
        .s3
        .put_object_stream_builder(format!("original_upload/{uuid}"))
        .execute_stream(&mut reader)
        .await
        .wrap_err("failed to upload to storage")?;

    if response.status_code() != 200 {
        return Err(eyre!(format!(
            "failed to upload to storage (status code: {})",
            response.status_code()
        ))
        .into());
    }

    sqlx::query!(
        "INSERT INTO recordings (id, uploaded_at, original_filename, original_s3_path) VALUES ($1, $2, $3, $4)",
        uuid,
        chrono::Utc::now(),
        filename,
        format!("original_upload/{uuid}")
    )
    .execute(&state.db)
    .await
    .wrap_err("failed to insert into database")?;

    // TODO: run this async
    analyze_recording(state, uuid)
        .await
        .wrap_err("failed to analyze")?;

    let mut headers = HeaderMap::new();
    headers.insert(
        "Location",
        format!("/recordings/{}", uuid).try_into().unwrap(),
    );

    Ok((
        StatusCode::CREATED,
        headers,
        Json(UploadResponse { upload_id: uuid }),
    ))
}

#[derive(serde::Serialize)]
pub struct UploadResponse {
    pub upload_id: uuid::Uuid,
}
