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
    let mut audio_filename: Option<String> = None;
    let mut transcript_path: Option<String> = None;
    let mut diarize = None;

    while let Some(field) = multipart.next_field().await? {
        let name = field.name().ok_or_eyre("field must have name")?.to_owned();

        if name == "audio" {
            let filename = field
                .file_name()
                .ok_or_eyre("uploaded audio file should have filename")?
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
                .wrap_err("failed to upload audio to storage")?;

            if response.status_code() != 200 {
                return Err(eyre!(format!(
                    "failed to upload audio to storage (status code: {})",
                    response.status_code()
                ))
                .into());
            }

            audio_filename = Some(filename);
        } else if name == "transcript" {
            if let Some(_) = field.file_name() {
                let mut reader =
                    tokio_util::io::StreamReader::new(field.map_err(|multipart_error| {
                        std::io::Error::new(std::io::ErrorKind::Other, multipart_error)
                    }));

                let response = state
                    .s3
                    .put_object_stream_builder(format!("original_transcript/{uuid}"))
                    .execute_stream(&mut reader)
                    .await
                    .wrap_err("failed to upload transcript to storage")?;

                if response.status_code() != 200 {
                    return Err(eyre!(format!(
                        "failed to upload transcript to storage (status code: {})",
                        response.status_code()
                    ))
                    .into());
                }

                transcript_path = Some(format!("original_transcript/{uuid}"));
            }
        } else if name == "diarize" {
            let text = field.text().await.unwrap_or_default();
            match text.as_str() {
                "true" => diarize = Some(true),
                "false" => diarize = Some(false),
                _ => diarize = None,
            }
        }
    }

    let audio_filename = if let Some(filename) = audio_filename {
        filename
    } else {
        return Err(eyre!("no audio file in upload").into());
    };

    sqlx::query!(
        "INSERT INTO recordings (id, uploaded_at, original_filename, original_s3_path, force_diarize, original_transcript_s3_path) VALUES ($1, $2, $3, $4, $5, $6)",
        uuid,
        chrono::Utc::now(),
        audio_filename,
        format!("original_upload/{uuid}"),
        diarize,
        transcript_path,
    )
    .execute(&state.db)
    .await
    .wrap_err("failed to insert into database")?;

    // TODO: use the extra fields (diarize, transcript_path) in analysis
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
