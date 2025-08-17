use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;
use thiserror::Error;

use crate::room::service::RoomService;
use crate::session::repository::SessionRepository;
use crate::session::service::SessionService;
use crate::websockets::ConnectionManager;
use crate::{event::EventBus, game::GameService, user::PlayerMappingService};

/// Shared application state containing all dependencies
#[derive(Clone)]
pub struct AppState {
    pub session_repository: Arc<dyn SessionRepository + Send + Sync>,
    pub session_service: Arc<SessionService>,
    pub room_service: Arc<RoomService>,
    pub event_bus: EventBus,
    pub connection_manager: Arc<dyn ConnectionManager>,
    pub game_service: Arc<GameService>,
    pub player_mapping: Arc<dyn PlayerMappingService>,
}

impl AppState {
    pub fn new(
        session_repository: Arc<dyn SessionRepository + Send + Sync>,
        session_service: Arc<SessionService>,
        room_service: Arc<RoomService>,
        event_bus: EventBus,
        connection_manager: Arc<dyn ConnectionManager>,
        game_service: Arc<GameService>,
        player_mapping: Arc<dyn PlayerMappingService>,
    ) -> Self {
        Self {
            session_repository,
            session_service,
            room_service,
            event_bus,
            connection_manager,
            game_service,
            player_mapping,
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

    #[error("Bad request: {0}")]
    BadRequest(String),

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
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
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
    use crate::room::repository::RoomRepository;
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
                    id: "dummy-room".to_string(),
                    host_uuid: Some("dummy-host".to_string()),
                    status: "ONLINE".to_string(),
                    player_uuids: vec!["dummy-host-uuid".to_string()],
                },
            ))
        }

        async fn leave_room(
            &self,
            _room_id: &str,
            _player_uuid: &str,
        ) -> Result<crate::room::repository::LeaveRoomResult, AppError> {
            // Always return success for dummy implementation
            Ok(crate::room::repository::LeaveRoomResult::PlayerNotInRoom)
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
        session_service: Option<Arc<SessionService>>,
        room_service: Option<Arc<RoomService>>,
        connection_manager: Option<Arc<dyn ConnectionManager>>,
        game_service: Option<Arc<GameService>>,
        player_mapping: Option<Arc<dyn PlayerMappingService>>,
    }

    impl AppStateBuilder {
        pub fn new() -> Self {
            Self {
                session_repository: None,
                session_service: None,
                room_service: None,
                connection_manager: None,
                game_service: None,
                player_mapping: None,
            }
        }

        pub fn with_session_repository(
            mut self,
            repo: Arc<dyn SessionRepository + Send + Sync>,
        ) -> Self {
            self.session_repository = Some(repo);
            self
        }

        pub fn with_session_service(mut self, service: Arc<SessionService>) -> Self {
            self.session_service = Some(service);
            self
        }

        pub fn with_room_service(mut self, service: Arc<RoomService>) -> Self {
            self.room_service = Some(service);
            self
        }

        /// Convenience method for tests that want to provide a room repository
        /// This creates a RoomService with the given repository
        pub fn with_room_repository(mut self, repo: Arc<dyn RoomRepository + Send + Sync>) -> Self {
            let player_mapping = self.player_mapping.clone().unwrap_or_else(|| {
                Arc::new(crate::user::mapping_service::InMemoryPlayerMappingService::new())
            });
            self.room_service = Some(Arc::new(RoomService::new(repo, player_mapping.clone())));
            // Ensure the player_mapping is stored for later use
            self.player_mapping = Some(player_mapping);
            self
        }

        pub fn with_connection_manager(mut self, manager: Arc<dyn ConnectionManager>) -> Self {
            self.connection_manager = Some(manager);
            self
        }

        pub fn with_game_service(mut self, service: Arc<GameService>) -> Self {
            self.game_service = Some(service);
            self
        }

        pub fn with_player_mapping(mut self, mapping: Arc<dyn PlayerMappingService>) -> Self {
            self.player_mapping = Some(mapping);
            self
        }

        pub fn build(self) -> AppState {
            let session_repository = self
                .session_repository
                .unwrap_or_else(|| Arc::new(DummySessionRepository));
            let player_mapping = self.player_mapping.clone().unwrap_or_else(|| {
                Arc::new(crate::user::mapping_service::InMemoryPlayerMappingService::new())
            });
            let session_service = self.session_service.unwrap_or_else(|| {
                Arc::new(SessionService::new(
                    session_repository.clone(),
                    player_mapping.clone(),
                ))
            });
            let room_service = self.room_service.unwrap_or_else(|| {
                let player_mapping = self.player_mapping.clone().unwrap_or_else(|| {
                    Arc::new(crate::user::mapping_service::InMemoryPlayerMappingService::new())
                });
                Arc::new(RoomService::new(
                    Arc::new(DummyRoomRepository),
                    player_mapping,
                ))
            });

            let game_service = self
                .game_service
                .unwrap_or_else(|| Arc::new(GameService::new()));

            AppState {
                session_repository,
                session_service,
                room_service,
                event_bus: EventBus::new(),
                connection_manager: self
                    .connection_manager
                    .unwrap_or_else(|| Arc::new(DummyConnectionManager)),
                game_service,
                player_mapping,
            }
        }
    }

    impl Default for AppStateBuilder {
        fn default() -> Self {
            Self::new()
        }
    }
}
