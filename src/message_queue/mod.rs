pub mod types;

use std::sync::Arc;

use eyre::Context;
use rdkafka::{
    Message,
    consumer::Consumer,
    producer::{FutureRecord, Producer},
    util::Timeout,
};

use crate::{AppState, message_queue::types::KafkaAnalysisResponse};

#[derive(Clone)]
pub struct KafkaKonnections {
    pub tx_tasks: rdkafka::producer::FutureProducer,
    pub rx_results: Arc<tokio::sync::Mutex<rdkafka::consumer::StreamConsumer>>,
}

impl KafkaKonnections {
    pub async fn send_request(&self, request: &types::AnalysisRequest) -> eyre::Result<()> {
        let bytes = serde_json::to_vec(request).unwrap();
        let record: FutureRecord<'_, (), Vec<u8>> =
            FutureRecord::to("analysis_requests").payload(&bytes);
        self.tx_tasks
            .send(record, None)
            .await
            .map_err(|v| v.0)
            .wrap_err("failed to send request to kafka")?;
        self.tx_tasks
            .flush(Timeout::Never)
            .wrap_err("failed to flush kafka")?;
        Ok(())
    }
}

pub async fn recv_loop(state: AppState) -> eyre::Result<()> {
    {
        let receiver = state.kafka.rx_results.lock().await;
        receiver
            .subscribe(&["metrics_output"])
            .wrap_err("failed to subscribe to topic 'metrics_output'")?;
        drop(receiver)
    }

    println!("waiting for messages");

    loop {
        tracing::debug!("waiting for receiver lock");
        let receiver = state.kafka.rx_results.lock().await;
        tracing::debug!("waiting for message");
        let recv = receiver.recv().await?;
        tracing::debug!("got message");
        let Some(content) = recv.payload() else {
            tracing::debug!("message had no content, skipping");
            continue;
        };
        println!("got message: {}", String::from_utf8_lossy(content));
        let response: KafkaAnalysisResponse = match serde_json::from_slice(content) {
            Ok(response) => response,
            Err(why) => {
                tracing::warn!("failed to deserialize message: {why}");
                continue;
            }
        };

        match response.data {
            types::KafkaAnalysisResponseInner::RecordingMetrics(recording_metrics) => {
                let mut tx = state.db.begin().await?;
                sqlx::query!(
                    "INSERT INTO recording_stats (recording_id, duration_seconds, sample_rate, channels, bit_depth, max_dbfs, rms, raw_data_length) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                    response.id,
                    recording_metrics.audio.duration_seconds,
                    recording_metrics.audio.sample_rate as i32,
                    recording_metrics.audio.channels as i32,
                    recording_metrics.audio.bit_depth as i32,
                    recording_metrics.audio.max_dbfs,
                    recording_metrics.audio.rms as f32,
                    recording_metrics.audio.raw_data_length as i64
                ).execute(&mut *tx)
                .await?;
                sqlx::query!(
                    "UPDATE recordings SET analysis_status='done', analysis_percent=100, analysis_last_update=now() WHERE id=$1",
                    response.id
                )
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
            }
            types::KafkaAnalysisResponseInner::ChannelMetrics(channel_metrics) => {
                let channel_id = uuid::Uuid::new_v4();
                let mut tx = state.db.begin().await?;
                sqlx::query!(
                    "INSERT INTO channels (id, recording, idx_in_file, wav2vec2_age_gender) VALUES ($1, $2, $3, $4)",
                    channel_id,
                    response.id,
                    channel_metrics.idx,
                    sqlx::types::Json(channel_metrics.age_gender) as _,
                )
                .execute(&mut *tx)
                .await?;

                for segment in channel_metrics.segments {
                    let segment_id = uuid::Uuid::new_v4();
                    sqlx::query!(
                        "INSERT INTO segments (id, channel, start_sec, end_sec, wav2vec2_emotion, whisper, content) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                        segment_id,
                        channel_id,
                        segment.whisper.start,
                        segment.whisper.end,
                        sqlx::types::Json(segment.emotion) as _,
                        sqlx::types::Json(segment.whisper.clone()) as _,
                        segment.whisper.text,
                    )
                    .execute(&mut *tx)
                    .await?;
                }
                sqlx::query!(
                    "UPDATE recordings SET analysis_status='running', analysis_last_update=now() WHERE id=$1",
                    response.id
                )
                .execute(&mut *tx)
                .await?;

                tx.commit().await?;
            }
            types::KafkaAnalysisResponseInner::ProgressMsg(progress_msg) => {
                if progress_msg.percent_done == 100 {
                    sqlx::query!(
                        "UPDATE recordings SET analysis_status='done', analysis_percent=100, analysis_last_update=now() WHERE id=$1",
                        response.id
                    )
                    .execute(&state.db)
                    .await?;
                } else {
                    sqlx::query!(
                    "UPDATE recordings SET analysis_status='running', analysis_percent=$1, analysis_last_update=now() WHERE id=$2",
                    progress_msg.percent_done,
                    response.id
                )
                .execute(&state.db)
                .await?;
                }
            }
            types::KafkaAnalysisResponseInner::ErrorMsg(error_msg) => {
                sqlx::query!(
                    "UPDATE recordings SET analysis_status='error', analysis_last_update=now(), analysis_error=$1 WHERE id=$2",
                    format!("{error_msg:?}"),
                    response.id
                )
                .execute(&state.db)
                .await?;
                println!("ERROR: {error_msg:?}")
            }
        }

        receiver.commit_message(&recv, rdkafka::consumer::CommitMode::Sync)?;
    }
}
