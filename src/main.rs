mod session;
mod shared;

use axum::{
    routing::{get, post},
    Router,
};
use session::repository::InMemorySessionRepository;
// use session::repository::PostgresSessionRepository; // For production
use shared::AppState;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "bigtwo=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Big Two game server");

    // Create shared application state with dependency injection
    // Easy to switch between implementations:
    let session_repository = Arc::new(InMemorySessionRepository::new());

    // For production with PostgreSQL:
    // let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    // let pool = sqlx::PgPool::connect(&database_url).await.expect("Failed to connect to database");
    // let session_repository = Arc::new(PostgresSessionRepository::new(pool));

    let app_state = AppState::new(session_repository);

    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/session", post(session::create_session))
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}
