use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;
use thiserror::Error;

use crate::event::EventBus;
use crate::room::repository::RoomRepository;
use crate::session::repository::SessionRepository;
use crate::websockets::ConnectionManager;

/// Shared application state containing all dependencies
#[derive(Clone)]
pub struct AppState {
    pub session_repository: Arc<dyn SessionRepository + Send + Sync>,
    pub room_repository: Arc<dyn RoomRepository + Send + Sync>,
    pub event_bus: EventBus,
    pub connection_manager: Arc<dyn ConnectionManager>,
}

impl AppState {
    pub fn new(
        session_repository: Arc<dyn SessionRepository + Send + Sync>,
        room_repository: Arc<dyn RoomRepository + Send + Sync>,
        event_bus: EventBus,
        connection_manager: Arc<dyn ConnectionManager>,
    ) -> Self {
        Self {
            session_repository,
            room_repository,
            event_bus,
            connection_manager,
        }
    }
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("JWT error: {0}")]
    JwtError(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal server error")]
    Internal,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::JwtError(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            AppError::DatabaseError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", msg),
            ),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        let body = Json(json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}

#[cfg(test)]
pub mod test_utils {
    use super::*;
    use crate::room::models::RoomModel;
    use crate::session::models::SessionModel;
    use async_trait::async_trait;

    /// Dummy session repository that does nothing - for tests that don't care about sessions
    pub struct DummySessionRepository;

    #[async_trait]
    impl SessionRepository for DummySessionRepository {
        async fn create_session(&self, _session: &SessionModel) -> Result<(), AppError> {
            Ok(())
        }
        async fn get_session(&self, _session_id: &str) -> Result<Option<SessionModel>, AppError> {
            Ok(None)
        }
        async fn update_session(&self, _session: &SessionModel) -> Result<(), AppError> {
            Ok(())
        }
        async fn delete_session(&self, _session_id: &str) -> Result<(), AppError> {
            Ok(())
        }
        async fn cleanup_expired_sessions(&self) -> Result<u64, AppError> {
            Ok(0)
        }
    }

    /// Dummy room repository that does nothing - for tests that don't care about rooms
    pub struct DummyRoomRepository;

    #[async_trait]
    impl RoomRepository for DummyRoomRepository {
        async fn create_room(&self, _room: &RoomModel) -> Result<(), AppError> {
            Ok(())
        }
        async fn get_room(&self, _room_id: &str) -> Result<Option<RoomModel>, AppError> {
            Ok(None)
        }
        async fn list_rooms(&self) -> Result<Vec<RoomModel>, AppError> {
            Ok(Vec::new())
        }
        async fn try_join_room(
            &self,
            _room_id: &str,
            _player_name: &str,
        ) -> Result<crate::room::repository::JoinRoomResult, AppError> {
            Ok(crate::room::repository::JoinRoomResult::Success(
                RoomModel {
                    id: _room_id.to_string(),
                    host_name: _player_name.to_string(),
                    status: "ONLINE".to_string(),
                    player_count: 1,
                },
            ))
        }
    }

    /// Dummy connection manager that does nothing - for tests that don't care about websockets
    pub struct DummyConnectionManager;

    #[async_trait]
    impl ConnectionManager for DummyConnectionManager {
        async fn add_connection(
            &self,
            _username: String,
            _sender: tokio::sync::mpsc::UnboundedSender<String>,
        ) {
        }

        async fn remove_connection(&self, _username: &str) {}

        async fn send_to_player(&self, _username: &str, _message: &str) {}

        async fn send_to_players(&self, _usernames: &[String], _message: &str) {}
    }

    /// Builder for creating AppState with overrides for testing
    pub struct AppStateBuilder {
        session_repository: Option<Arc<dyn SessionRepository + Send + Sync>>,
        room_repository: Option<Arc<dyn RoomRepository + Send + Sync>>,
        connection_manager: Option<Arc<dyn ConnectionManager>>,
    }

    impl AppStateBuilder {
        pub fn new() -> Self {
            Self {
                session_repository: None,
                room_repository: None,
                connection_manager: None,
            }
        }

        pub fn with_session_repository(
            mut self,
            repo: Arc<dyn SessionRepository + Send + Sync>,
        ) -> Self {
            self.session_repository = Some(repo);
            self
        }

        pub fn with_room_repository(mut self, repo: Arc<dyn RoomRepository + Send + Sync>) -> Self {
            self.room_repository = Some(repo);
            self
        }

        pub fn with_connection_manager(mut self, manager: Arc<dyn ConnectionManager>) -> Self {
            self.connection_manager = Some(manager);
            self
        }

        pub fn build(self) -> AppState {
            AppState {
                session_repository: self
                    .session_repository
                    .unwrap_or_else(|| Arc::new(DummySessionRepository)),
                room_repository: self
                    .room_repository
                    .unwrap_or_else(|| Arc::new(DummyRoomRepository)),
                event_bus: EventBus::new(),
                connection_manager: self
                    .connection_manager
                    .unwrap_or_else(|| Arc::new(DummyConnectionManager)),
            }
        }
    }

    impl Default for AppStateBuilder {
        fn default() -> Self {
            Self::new()
        }
    }
}
