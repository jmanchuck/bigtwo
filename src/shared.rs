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

/// Shared application state containing all dependencies
#[derive(Clone)]
pub struct AppState {
    pub session_repository: Arc<dyn SessionRepository + Send + Sync>,
    pub room_repository: Arc<dyn RoomRepository + Send + Sync>,
    pub event_bus: EventBus,
}

impl AppState {
    pub fn new(
        session_repository: Arc<dyn SessionRepository + Send + Sync>,
        room_repository: Arc<dyn RoomRepository + Send + Sync>,
        event_bus: EventBus,
    ) -> Self {
        Self {
            session_repository,
            room_repository,
            event_bus,
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

    /// Builder for creating AppState with overrides for testing
    pub struct AppStateBuilder {
        session_repository: Option<Arc<dyn SessionRepository + Send + Sync>>,
        room_repository: Option<Arc<dyn RoomRepository + Send + Sync>>,
    }

    impl AppStateBuilder {
        pub fn new() -> Self {
            Self {
                session_repository: None,
                room_repository: None,
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

        pub fn build(self) -> AppState {
            AppState {
                session_repository: self
                    .session_repository
                    .unwrap_or_else(|| Arc::new(DummySessionRepository)),
                room_repository: self
                    .room_repository
                    .unwrap_or_else(|| Arc::new(DummyRoomRepository)),
                event_bus: EventBus::new(1000),
            }
        }
    }

    impl Default for AppStateBuilder {
        fn default() -> Self {
            Self::new()
        }
    }
}
