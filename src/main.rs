mod event;
mod game;
mod room;
mod session;
mod shared;
mod websockets;

use axum::{
    http::{HeaderValue, Method},
    middleware,
    routing::{get, post},
    Router,
};
use room::repository::InMemoryRoomRepository;
use session::repository::{
    InMemorySessionRepository, PostgresSessionRepository, SessionRepository,
};
use session::service::SessionService;
use shared::AppState;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::websockets::InMemoryConnectionManager;
use crate::{event::EventBus, game::GameManager};

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
    // Smart configuration: Use PostgreSQL if DATABASE_URL is set, otherwise in-memory
    let session_repository: Arc<dyn SessionRepository + Send + Sync> =
        if let Ok(database_url) = std::env::var("DATABASE_URL") {
            info!("Using PostgreSQL session storage (persistent across restarts)");
            match sqlx::PgPool::connect(&database_url).await {
                Ok(pool) => {
                    info!("✅ Connected to PostgreSQL successfully");
                    Arc::new(PostgresSessionRepository::new(pool))
                }
                Err(e) => {
                    warn!("❌ Failed to connect to PostgreSQL: {}", e);
                    info!("🔄 Falling back to in-memory session storage");
                    Arc::new(InMemorySessionRepository::new())
                }
            }
        } else {
            info!("Using in-memory session storage (sessions lost on restart)");
            info!("💡 Set DATABASE_URL to use PostgreSQL for persistent sessions");
            Arc::new(InMemorySessionRepository::new())
        };

    let session_service = Arc::new(SessionService::new(session_repository.clone()));
    let room_repository = Arc::new(InMemoryRoomRepository::new());
    let event_bus = EventBus::new();
    let connection_manager = Arc::new(InMemoryConnectionManager::new());
    let game_manager = Arc::new(GameManager::new());

    let app_state = AppState::new(
        session_repository,
        session_service,
        room_repository,
        event_bus,
        connection_manager,
        game_manager,
    );

    // Configure CORS for development
    let cors = CorsLayer::new()
        .allow_origin([
            "https://localhost:5173".parse::<HeaderValue>().unwrap(),
            "http://localhost:5173".parse::<HeaderValue>().unwrap(),
        ])
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ]);

    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/session", post(session::create_session))
        .route(
            "/session/validate",
            get(session::validate_session).layer(middleware::from_fn_with_state(
                app_state.clone(),
                session::jwt_auth,
            )),
        )
        .route("/room", post(room::create_room))
        .route("/rooms", get(room::list_rooms))
        .route(
            "/room/:room_id/join",
            post(room::join_room).layer(middleware::from_fn_with_state(
                app_state.clone(),
                session::jwt_auth,
            )),
        )
        .route("/room/:room_id", get(room::get_room_details))
        .route("/ws/:room_id", get(websockets::websocket_handler))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}
