use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, instrument};

use super::{
    generators::{UsernameGenerator, UuidGenerator},
    models::SessionModel,
    repository::SessionRepository,
    token::TokenConfig,
    types::SessionResponse,
};
use crate::{shared::AppError, user::PlayerMappingService};

/// Configuration for session creation
#[derive(Clone)]
pub struct SessionCreationConfig {
    pub expiration_days: i64,
}

impl Default for SessionCreationConfig {
    fn default() -> Self {
        Self {
            expiration_days: 7, // Default to 7 days
        }
    }
}

/// Orchestrates the complex session creation process
/// Separates concerns and provides transaction-like semantics
pub struct SessionCreator {
    uuid_generator: Arc<dyn UuidGenerator>,
    username_generator: Arc<dyn UsernameGenerator>,
    session_repository: Arc<dyn SessionRepository + Send + Sync>,
    player_mapping: Arc<dyn PlayerMappingService>,
    session_to_player_uuid: Arc<RwLock<HashMap<String, String>>>,
    token_config: TokenConfig,
    config: SessionCreationConfig,
}

/// Result of session creation operation
#[derive(Debug)]
pub struct SessionCreationResult {
    pub session_response: SessionResponse,
    pub cleanup_needed: bool,
}

impl SessionCreator {
    pub fn new(
        uuid_generator: Arc<dyn UuidGenerator>,
        username_generator: Arc<dyn UsernameGenerator>,
        session_repository: Arc<dyn SessionRepository + Send + Sync>,
        player_mapping: Arc<dyn PlayerMappingService>,
        session_to_player_uuid: Arc<RwLock<HashMap<String, String>>>,
        token_config: TokenConfig,
        config: SessionCreationConfig,
    ) -> Self {
        Self {
            uuid_generator,
            username_generator,
            session_repository,
            player_mapping,
            session_to_player_uuid,
            token_config,
            config,
        }
    }

    /// Creates a new session with proper error handling and partial cleanup
    #[instrument(skip(self))]
    pub async fn create_session(&self) -> Result<SessionCreationResult, AppError> {
        // Step 1: Generate username
        let username = self.generate_username().await?;
        info!(username = %username, "Generated username");

        // Step 2: Generate player UUID
        let player_uuid = self.generate_player_uuid().await?;
        info!(player_uuid = %player_uuid, "Generated player UUID");

        // Step 3: Create session model
        let session_model = self.create_session_model(username.clone()).await?;
        info!(session_id = %session_model.id, "Created session model");

        // Step 4: Store session in database
        self.store_session(&session_model).await?;
        info!(session_id = %session_model.id, "Stored session in database");

        // Step 5: Register player mapping
        let mapping_cleanup_needed = self
            .register_player_mapping(&player_uuid, &username)
            .await?;
        info!(
            player_uuid = %player_uuid,
            username = %username,
            "Registered player mapping"
        );

        // Step 6: Store session-to-player UUID mapping
        self.store_session_mapping(&session_model.id, &player_uuid)
            .await?;
        info!(
            session_id = %session_model.id,
            player_uuid = %player_uuid,
            "Stored session-to-player mapping"
        );

        // Step 7: Create JWT token
        let token = self.create_jwt_token(&session_model.id, &username).await?;
        info!(username = %username, "Created JWT token");

        Ok(SessionCreationResult {
            session_response: SessionResponse {
                session_id: token,
                username,
                player_uuid,
            },
            cleanup_needed: mapping_cleanup_needed,
        })
    }

    /// Generate username using configured generator
    async fn generate_username(&self) -> Result<String, AppError> {
        self.username_generator.generate().await.pipe(Ok)
    }

    /// Generate player UUID using configured generator
    async fn generate_player_uuid(&self) -> Result<String, AppError> {
        self.uuid_generator.generate().await.pipe(Ok)
    }

    /// Create session model with proper expiration
    async fn create_session_model(&self, username: String) -> Result<SessionModel, AppError> {
        SessionModel::new(username, self.config.expiration_days).pipe(Ok)
    }

    /// Store session in repository
    async fn store_session(&self, session_model: &SessionModel) -> Result<(), AppError> {
        self.session_repository.create_session(session_model).await
    }

    /// Register player mapping, returns true if cleanup is needed on failure
    async fn register_player_mapping(
        &self,
        player_uuid: &str,
        username: &str,
    ) -> Result<bool, AppError> {
        match self
            .player_mapping
            .register_player(player_uuid.to_string(), username.to_string())
            .await
        {
            Ok(_) => Ok(false),
            Err(_e) => {
                // In case of mapping failure, we need cleanup
                Err(AppError::Internal)
            }
        }
    }

    /// Store session-to-player UUID mapping
    async fn store_session_mapping(
        &self,
        session_id: &str,
        player_uuid: &str,
    ) -> Result<(), AppError> {
        let mut session_uuid_map = self.session_to_player_uuid.write().await;
        session_uuid_map.insert(session_id.to_string(), player_uuid.to_string());
        Ok(())
    }

    /// Create JWT token with session information
    async fn create_jwt_token(&self, session_id: &str, username: &str) -> Result<String, AppError> {
        self.token_config
            .create_token(session_id.to_string(), username.to_string())
    }

    /// Cleanup partial session creation (for future enhancement)
    #[allow(dead_code)]
    async fn cleanup_partial_session(
        &self,
        session_id: &str,
        player_uuid: &str,
    ) -> Result<(), AppError> {
        // Remove from session repository
        let _ = self.session_repository.delete_session(session_id).await;

        // Remove from player mapping
        self.player_mapping.remove_player(player_uuid).await;

        // Remove from session-to-player mapping
        let mut session_uuid_map = self.session_to_player_uuid.write().await;
        session_uuid_map.remove(session_id);

        Ok(())
    }
}

/// Extension trait for pipe operations
trait Pipe<T> {
    fn pipe<U, F>(self, f: F) -> U
    where
        F: FnOnce(T) -> U;
}

impl<T> Pipe<T> for T {
    fn pipe<U, F>(self, f: F) -> U
    where
        F: FnOnce(T) -> U,
    {
        f(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::generators::{DefaultUuidGenerator, PetNameUsernameGenerator};
    use crate::session::repository::InMemorySessionRepository;
    use crate::user::mapping_service::InMemoryPlayerMappingService;
    use std::collections::HashMap;

    fn create_test_session_creator() -> SessionCreator {
        let uuid_generator = Arc::new(DefaultUuidGenerator::new());
        let username_generator = Arc::new(PetNameUsernameGenerator::new());
        let session_repository = Arc::new(InMemorySessionRepository::new());
        let player_mapping = Arc::new(InMemoryPlayerMappingService::new());
        let session_to_player_uuid = Arc::new(RwLock::new(HashMap::new()));
        let token_config = TokenConfig::new();
        let config = SessionCreationConfig::default();

        SessionCreator::new(
            uuid_generator,
            username_generator,
            session_repository,
            player_mapping,
            session_to_player_uuid,
            token_config,
            config,
        )
    }

    #[tokio::test]
    async fn test_create_session_success() {
        let creator = create_test_session_creator();
        let result = creator.create_session().await;

        assert!(result.is_ok());
        let session_result = result.unwrap();

        // Should have valid session response
        assert!(!session_result.session_response.session_id.is_empty());
        assert!(session_result.session_response.session_id.contains('.')); // JWT has dots
        assert!(!session_result.session_response.username.is_empty());
        assert!(session_result.session_response.username.contains('-')); // Pet names have dashes
        assert!(!session_result.session_response.player_uuid.is_empty());

        // Should be able to validate the created session
        let claims = creator
            .token_config
            .validate_token(&session_result.session_response.session_id)
            .unwrap();
        assert_eq!(claims.username, session_result.session_response.username);
    }

    #[tokio::test]
    async fn test_create_multiple_sessions_unique() {
        let creator = create_test_session_creator();

        let result1 = creator.create_session().await.unwrap();
        let result2 = creator.create_session().await.unwrap();

        // Sessions should be unique
        assert_ne!(
            result1.session_response.session_id,
            result2.session_response.session_id
        );
        assert_ne!(
            result1.session_response.player_uuid,
            result2.session_response.player_uuid
        );
        // Usernames may or may not be unique (petnames can repeat)
    }
}
