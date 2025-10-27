use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Database model for user sessions table
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SessionModel {
    pub id: String,       // UUID v4 as string (also serves as player identifier)
    pub username: String, // Auto-generated pet name
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_accessed: Option<DateTime<Utc>>, // When session was last used (nullable for existing rows)
}

impl SessionModel {
    /// Creates a new session model with generated ID and timestamps
    /// The session ID also serves as the player identifier
    pub fn new(username: String, expiration_days: i64) -> Self {
        let now = Utc::now();
        let expires_at = now + chrono::Duration::days(expiration_days);

        Self {
            id: Uuid::new_v4().to_string(),
            username,
            created_at: now,
            expires_at,
            last_accessed: Some(now),
        }
    }

    /// Checks if the session has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Extends the session expiration by the given number of days
    #[allow(dead_code)] // Public API for session management
    pub fn extend_expiration(&mut self, days: i64) {
        self.expires_at = Utc::now() + chrono::Duration::days(days);
    }

    /// Updates the last accessed timestamp
    #[allow(dead_code)] // Reserved for session activity tracking
    pub fn touch(&mut self) {
        self.last_accessed = Some(Utc::now());
    }

    /// Gets the last accessed time, defaulting to created_at if never accessed
    #[allow(dead_code)] // Reserved for session activity tracking
    pub fn get_last_accessed(&self) -> DateTime<Utc> {
        self.last_accessed.unwrap_or(self.created_at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_session_model() {
        let username = "test-user".to_string();
        let session = SessionModel::new(username.clone(), 7);

        assert_eq!(session.username, username);
        assert!(!session.id.is_empty());
        assert!(session.expires_at > session.created_at);
        assert!(!session.is_expired());
    }

    #[test]
    fn test_session_expiration() {
        let mut session = SessionModel::new("test".to_string(), -1); // Expired
        assert!(session.is_expired());

        session.extend_expiration(7);
        assert!(!session.is_expired());
    }
}
