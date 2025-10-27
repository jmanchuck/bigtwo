use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use tracing::{debug, instrument};

use super::types::SessionClaims;
use crate::shared::AppError;

/// Configuration for JWT token operations
#[derive(Clone)]
pub struct TokenConfig {
    secret: String,
    pub expiration_days: i64,
}

impl TokenConfig {
    pub fn new() -> Self {
        // Allow configuring expiration via env var, default to 365 days (1 year)
        let expiration_days = std::env::var("SESSION_EXPIRATION_DAYS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(365);

        Self {
            secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "your-secret-key-change-in-production".to_string()),
            expiration_days,
        }
    }

    /// Creates a new JWT token with the given session data
    #[instrument(skip(self, session_id, username))]
    pub fn create_token(&self, session_id: String, username: String) -> Result<String, AppError> {
        let now = Utc::now();
        let exp = (now + Duration::days(self.expiration_days)).timestamp() as usize;

        debug!(
            expiration_days = self.expiration_days,
            exp_timestamp = exp,
            "Creating JWT token with expiration"
        );

        let claims = SessionClaims {
            session_id,
            username,
            exp,
            iat: now.timestamp() as usize,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_ref()),
        )
        .map_err(|e| {
            debug!(error = %e, "Failed to encode JWT token");
            AppError::JwtError(e.to_string())
        })
    }

    /// Validates a JWT token and returns the claims if valid
    #[instrument(skip(self, token))]
    pub fn validate_token(&self, token: &str) -> Result<SessionClaims, AppError> {
        debug!("Decoding and validating JWT token");

        decode::<SessionClaims>(
            token,
            &DecodingKey::from_secret(self.secret.as_ref()),
            &Validation::default(),
        )
        .map(|data| {
            debug!(
                username = %data.claims.username,
                session_id = %data.claims.session_id,
                exp = data.claims.exp,
                "JWT token decoded successfully"
            );
            data.claims
        })
        .map_err(|e| {
            debug!(error = %e, "Failed to decode JWT token");
            AppError::JwtError(e.to_string())
        })
    }
}

impl Default for TokenConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_validate_token() {
        let config = TokenConfig::new();
        let session_id = "test-session-id".to_string();
        let username = "test-user".to_string();

        // Create token
        let token = config
            .create_token(session_id.clone(), username.clone())
            .unwrap();
        assert!(!token.is_empty());

        // Validate token
        let claims = config.validate_token(&token).unwrap();
        assert_eq!(claims.session_id, session_id);
        assert_eq!(claims.username, username);
        assert!(claims.exp > claims.iat);
    }

    #[test]
    fn test_invalid_token() {
        let config = TokenConfig::new();
        let result = config.validate_token("invalid.token.here");
        assert!(matches!(result, Err(AppError::JwtError(_))));
    }

    #[test]
    fn test_token_with_different_secret() {
        let config1 = TokenConfig::new();
        let config2 = TokenConfig::new();

        // Create token with first config
        let token = config1
            .create_token("session".to_string(), "user".to_string())
            .unwrap();

        // Should validate with same config
        assert!(config1.validate_token(&token).is_ok());

        // Should also validate with second config (same secret in test)
        assert!(config2.validate_token(&token).is_ok());
    }
}
