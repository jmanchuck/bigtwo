use serde::{Deserialize, Serialize};

/// JWT claims structure containing session information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionClaims {
    pub session_id: String,
    pub username: String,
    pub exp: usize, // Expiration timestamp (standard JWT claim)
    pub iat: usize, // Issued at timestamp (standard JWT claim)
}

/// Response structure for session creation endpoint
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SessionResponse {
    pub session_id: String, // The JWT token
    pub username: String,
    pub player_uuid: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_session_claims_serialization() {
        let claims = SessionClaims {
            session_id: "test-id".to_string(),
            username: "test-user".to_string(),
            exp: 1234567890,
            iat: 1234567800,
        };

        // Should serialize to JSON
        let json = serde_json::to_string(&claims).unwrap();
        assert!(json.contains("test-id"));
        assert!(json.contains("test-user"));

        // Should deserialize from JSON
        let deserialized: SessionClaims = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, claims);
    }

    #[test]
    fn test_session_response_serialization() {
        let response = SessionResponse {
            session_id: "jwt-token-here".to_string(),
            username: "happy-cat".to_string(),
            player_uuid: "player-uuid".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("jwt-token-here"));
        assert!(json.contains("happy-cat"));
        assert!(json.contains("player-uuid"));
    }
}
