pub mod analysis_submit;
pub mod endpoints;
pub mod message_queue;
pub mod result;
pub mod url;

use std::{env::var, sync::Arc};

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post, put},
};
use rdkafka::config::RDKafkaLogLevel;
use tower_http::cors::CorsLayer;

use crate::message_queue::KafkaKonnections;

#[derive(Clone)]
pub struct AppState {
    db: sqlx::PgPool,
    s3: s3::Bucket,
    kafka: KafkaKonnections,
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt::init();

    let url = var("DATABASE_URL").expect("DATABASE_URL should be set to a postgres:// schema");

    let mut conf = rdkafka::ClientConfig::new();
    conf.set("bootstrap.servers", "localhost:9092");
    conf.set("message.timeout.ms", "5000");
    conf.set_log_level(RDKafkaLogLevel::Debug);

    let konn = KafkaKonnections {
        tx_tasks: conf
            .clone()
            .create()
            .expect("should be able to create kafka producer"),
        rx_results: Arc::new(tokio::sync::Mutex::new(
            conf.clone()
                .set("group.id", "backend-app")
                .set("allow.auto.create.topics", "true")
                .create()
                .expect("should be able to create kafka consumer"),
        )),
    };

    let state = AppState {
        db: sqlx::PgPool::connect(&url)
            .await
            .expect("should be able to connect to database"),
        s3: *s3::Bucket::new(
            &var("AWS_BUCKET").expect("AWS_BUCKET must be set"),
            s3::Region::Custom {
                region: var("AWS_REGION").expect("AWS_REGION must be set"),
                endpoint: var("AWS_ENDPOINT_URL").expect("AWS_ENDPOINT_URL must be set"),
            },
            s3::creds::Credentials::new(
                Some(&var("AWS_ACCESS_KEY_ID").expect("AWS_ACCESS_KEY_ID must be set")),
                Some(&var("AWS_SECRET_ACCESS_KEY").expect("AWS_SECRET_ACCESS_KEY must be set")),
                None,
                None,
                None,
            )
            .expect("s3 credentials should be valid"),
        )
        .expect("s3 bucket should be valid")
        .with_path_style(),
        kafka: konn,
    };

    tokio::spawn({
        let state = state.clone();
        async move { message_queue::recv_loop(state.clone()).await.unwrap() }
    });

    sqlx::migrate!()
        .run(&state.db)
        .await
        .expect("should be able to run migrations");

    let app = axum::Router::new()
        .route("/", get(index))
        .route("/upload", post(endpoints::upload::upload_audio_file))
        .route("/recordings", get(endpoints::recording::list_recordings))
        .route(
            "/recordings/{id}",
            get(endpoints::recording::get_recording).delete(endpoints::recording::delete_recording),
        )
        .route("/channels/{id}", get(endpoints::channel::get_channel))
        .route(
            "/channels/{id}/assigned_name",
            put(endpoints::channel::set_assigned_name),
        )
        .route(
            "/channels/{id}/segments",
            get(endpoints::segment::get_segments),
        )
        .with_state(state)
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024 * 1024))
        .layer(CorsLayer::very_permissive());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("should be able to bind to port 3000");
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn index() -> &'static str {
    "hello world"
}
