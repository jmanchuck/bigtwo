mod cards;
mod game;
mod lobby;
mod room;
mod session;
mod shared;
mod websockets;

use axum::{
    routing::{get, post},
    Router,
};
use room::repository::InMemoryRoomRepository;
use session::repository::InMemorySessionRepository;
// use session::repository::PostgresSessionRepository; // For production
use axum::http::{HeaderValue, Method};
use shared::AppState;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "bigtwo=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Big Two game server");

    // Create shared application state with dependency injection
    // Easy to switch between implementations:
    let session_repository = Arc::new(InMemorySessionRepository::new());
    let room_repository = Arc::new(InMemoryRoomRepository::new());

    // For production with PostgreSQL:
    // let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    // let pool = sqlx::PgPool::connect(&database_url).await.expect("Failed to connect to database");
    // let session_repository = Arc::new(PostgresSessionRepository::new(pool));

    let app_state = AppState::new(session_repository, room_repository);

    // Configure CORS for development
    let cors = CorsLayer::new()
        .allow_origin([
            "https://localhost:5173".parse::<HeaderValue>().unwrap(),
            "http://localhost:5173".parse::<HeaderValue>().unwrap(),
        ])
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderName::from_static("x-session-id"),
        ]);

    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/session", post(session::create_session))
        .route("/room", post(room::create_room))
        .route("/rooms", get(room::list_rooms))
        .route("/ws/:room_id", get(websockets::websocket_handler))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}
