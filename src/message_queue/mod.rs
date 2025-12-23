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
        tracing::info!("sending request to kafka: {request:?}");
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

        let Some(_) = sqlx::query!("SELECT id FROM recordings WHERE id=$1", response.id)
            .fetch_optional(&state.db)
            .await?
        else {
            tracing::warn!("recording not found, skipping");
            continue;
        };

        match response.data {
            types::KafkaAnalysisResponseInner::RecordingMetrics(recording_metrics) => {
                let mut tx = state.db.begin().await?;
                sqlx::query!(
                    "INSERT INTO recording_stats (recording_id, metrics_list) VALUES ($1, $2)",
                    response.id,
                    sqlx::types::Json(recording_metrics.metrics) as _
                )
                .execute(&mut *tx)
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
                    "INSERT INTO channels (id, recording, idx_in_file, metrics_list) VALUES ($1, $2, $3, $4)",
                    channel_id,
                    response.id,
                    channel_metrics.idx,
                    sqlx::types::Json(channel_metrics.metrics) as _,
                )
                .execute(&mut *tx)
                .await?;

                for segment in channel_metrics.segments {
                    let segment_id = uuid::Uuid::new_v4();
                    sqlx::query!(
                        "INSERT INTO segments (id, channel, start_sec, end_sec, content, metrics_list) VALUES ($1, $2, $3, $4, $5, $6)",
                        segment_id,
                        channel_id,
                        segment.start,
                        segment.end,
                        segment.text,
                        sqlx::types::Json(segment.metrics) as _,
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
                let existing_progress = sqlx::query!(
                    "SELECT analysis_percent FROM recordings WHERE id=$1",
                    response.id
                )
                .fetch_optional(&state.db)
                .await?
                .map(|row| row.analysis_percent);

                if let Some(existing_progress) = existing_progress {
                    if existing_progress > progress_msg.percent_done {
                        continue;
                    }
                }

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
