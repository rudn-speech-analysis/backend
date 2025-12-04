use axum::routing::get;

#[derive(Clone)]
pub struct AppState {
    db: sqlx::PgPool,
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt::init();

    let url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL should be set to a postgres:// schema");
    let state = AppState {
        db: sqlx::PgPool::connect(&url)
            .await
            .expect("should be able to connect to database"),
    };

    sqlx::migrate!()
        .run(&state.db)
        .await
        .expect("should be able to run migrations");

    let app = axum::Router::new().route("/", get(index)).with_state(state);

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
