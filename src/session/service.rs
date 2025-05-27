use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

use super::{
    models::SessionModel,
    repository::SessionRepository,
    token::TokenConfig,
    types::{SessionClaims, SessionResponse},
};
use crate::shared::AppError;

/// Service for handling session business logic
pub struct SessionService {
    token_config: TokenConfig,
    repository: Arc<dyn SessionRepository + Send + Sync>,
}

impl SessionService {
    pub fn new(repository: Arc<dyn SessionRepository + Send + Sync>) -> Self {
        Self {
            token_config: TokenConfig::new(),
            repository,
        }
    }

    /// Creates a new session with a generated username and JWT token
    #[instrument(skip(self))]
    pub async fn create_session(&self) -> Result<SessionResponse, AppError> {
        let username = self.generate_username();
        debug!(username = %username, "Generated username");

        // Create session model for database
        let session_model = SessionModel::new(username.clone(), self.token_config.expiration_days);
        debug!(session_id = %session_model.id, "Generated session ID");

        // Store session in database
        self.repository.create_session(&session_model).await?;

        // Create JWT token with session ID
        let token = self
            .token_config
            .create_token(session_model.id, username.clone())?;
        info!(username = %username, "JWT token created successfully");

        Ok(SessionResponse {
            session_id: token,
            username,
        })
    }

    /// Validates a session token and returns the claims if valid
    #[instrument(skip(self, token))]
    pub async fn validate_session(&self, token: &str) -> Result<SessionClaims, AppError> {
        debug!("Validating session token");

        // First validate JWT token structure and signature
        let claims = self.token_config.validate_token(token)?;
        debug!(
            username = %claims.username,
            session_id = %claims.session_id,
            "JWT token structure validated"
        );

        // Then validate session exists in database and hasn't been revoked
        match self.repository.get_session(&claims.session_id).await? {
            Some(session_model) => {
                if session_model.is_expired() {
                    warn!(
                        session_id = %claims.session_id,
                        "Session found in database but has expired"
                    );
                    return Err(AppError::Unauthorized("Session has expired".to_string()));
                }

                info!(
                    username = %claims.username,
                    session_id = %claims.session_id,
                    "Session validated successfully against database"
                );

                Ok(claims)
            }
            None => {
                warn!(
                    session_id = %claims.session_id,
                    "Session not found in database - may have been revoked"
                );
                Err(AppError::Unauthorized(
                    "Session not found or has been revoked".to_string(),
                ))
            }
        }
    }

    /// Revokes a session by removing it from the database
    #[instrument(skip(self))]
    pub async fn revoke_session(&self, session_id: &str) -> Result<(), AppError> {
        debug!(session_id = %session_id, "Revoking session");

        self.repository.delete_session(session_id).await?;

        info!(session_id = %session_id, "Session revoked successfully");
        Ok(())
    }

    /// Extends a session's expiration time
    #[instrument(skip(self))]
    pub async fn extend_session(&self, session_id: &str) -> Result<(), AppError> {
        debug!(session_id = %session_id, "Extending session expiration");

        let mut session = self
            .repository
            .get_session(session_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Session not found".to_string()))?;

        session.extend_expiration(self.token_config.expiration_days);
        self.repository.update_session(&session).await?;

        info!(session_id = %session_id, "Session expiration extended successfully");
        Ok(())
    }

    /// Cleans up expired sessions from the database
    #[instrument(skip(self))]
    pub async fn cleanup_expired_sessions(&self) -> Result<u64, AppError> {
        debug!("Starting cleanup of expired sessions");

        let removed_count = self.repository.cleanup_expired_sessions().await?;

        info!(
            removed_sessions = removed_count,
            "Expired sessions cleanup completed"
        );
        Ok(removed_count)
    }

    /// Generates a random pet name for the user
    fn generate_username(&self) -> String {
        petname::Petnames::default().generate_one(2, "-")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::repository::InMemorySessionRepository;

    #[tokio::test]
    async fn test_create_session() {
        let repo = Arc::new(InMemorySessionRepository::new());
        let service = SessionService::new(repo);
        let result = service.create_session().await;

        assert!(result.is_ok());
        let session = result.unwrap();

        // Should have a token (JWT)
        assert!(!session.session_id.is_empty());
        assert!(session.session_id.contains('.')); // JWT has dots

        // Should have a username
        assert!(!session.username.is_empty());
        assert!(session.username.contains('-')); // Pet names have dashes
    }

    #[tokio::test]
    async fn test_validate_session_success() {
        let repo = Arc::new(InMemorySessionRepository::new());
        let service = SessionService::new(repo.clone());

        // Create a session
        let session_response = service.create_session().await.unwrap();

        // Validate the session
        let claims = service
            .validate_session(&session_response.session_id)
            .await
            .unwrap();
        assert_eq!(claims.username, session_response.username);
    }

    #[tokio::test]
    async fn test_validate_session_not_found() {
        let repo = Arc::new(InMemorySessionRepository::new());
        let service = SessionService::new(repo);

        // Create a token manually (not in database)
        let token_config = TokenConfig::new();
        let token = token_config
            .create_token("non-existent-session".to_string(), "test-user".to_string())
            .unwrap();

        // Should fail validation because session is not in database
        let result = service.validate_session(&token).await;
        assert!(matches!(result, Err(AppError::Unauthorized(_))));
    }

    #[tokio::test]
    async fn test_revoke_session() {
        let repo = Arc::new(InMemorySessionRepository::new());
        let service = SessionService::new(repo.clone());

        // Create a session
        let session_response = service.create_session().await.unwrap();
        let claims = service
            .validate_session(&session_response.session_id)
            .await
            .unwrap();

        // Revoke the session
        service.revoke_session(&claims.session_id).await.unwrap();

        // Should fail validation after revocation
        let result = service.validate_session(&session_response.session_id).await;
        assert!(matches!(result, Err(AppError::Unauthorized(_))));
    }
}
