use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Service for managing UUID → playername mappings
/// This enables internal systems to use UUIDs while preserving playername
/// for external API contracts and UI interactions
///
/// Note: Only UUID → playername mapping is supported since playername uniqueness
/// is not guaranteed across sessions
#[async_trait]
pub trait PlayerMappingService: Send + Sync {
    /// Register a new player with UUID and playername
    async fn register_player(&self, uuid: String, playername: String) -> Result<(), MappingError>;

    /// Get playername by UUID
    async fn get_playername(&self, uuid: &str) -> Option<String>;

    /// Remove player mapping (when session expires)
    async fn remove_player(&self, uuid: &str) -> bool;

    /// Get all active player mappings (for debugging/monitoring)
    async fn get_all_mappings(&self) -> Vec<(String, String)>; // (uuid, playername) pairs
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum MappingError {
    #[error("Player with UUID {uuid} already exists with playername: {existing_playername}")]
    UuidAlreadyExists {
        uuid: String,
        existing_playername: String,
    },

    #[error("Invalid UUID format: {uuid}")]
    InvalidUuid { uuid: String },
}

/// In-memory implementation of PlayerMappingService
/// Uses RwLock for concurrent access with read optimization
pub struct InMemoryPlayerMappingService {
    uuid_to_playername: Arc<RwLock<HashMap<String, String>>>,
}

impl InMemoryPlayerMappingService {
    pub fn new() -> Self {
        Self {
            uuid_to_playername: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Generate a new UUID for a player
    pub fn generate_uuid() -> String {
        Uuid::new_v4().to_string()
    }

    /// Validate UUID format
    fn validate_uuid(uuid: &str) -> Result<(), MappingError> {
        let uuid_to_validate = uuid.strip_prefix("bot-").unwrap_or(uuid);

        Uuid::parse_str(uuid_to_validate).map_err(|_| MappingError::InvalidUuid {
            uuid: uuid.to_string(),
        })?;
        Ok(())
    }
}

impl Default for InMemoryPlayerMappingService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PlayerMappingService for InMemoryPlayerMappingService {
    async fn register_player(&self, uuid: String, playername: String) -> Result<(), MappingError> {
        // Validate UUID format
        Self::validate_uuid(&uuid)?;

        // Acquire write lock
        let mut uuid_map = self.uuid_to_playername.write().await;

        // Check for existing UUID
        if let Some(existing_playername) = uuid_map.get(&uuid) {
            return Err(MappingError::UuidAlreadyExists {
                uuid,
                existing_playername: existing_playername.clone(),
            });
        }

        // Insert mapping
        uuid_map.insert(uuid.clone(), playername.clone());

        info!(
            uuid = %uuid,
            playername = %playername,
            "Registered new player mapping"
        );

        Ok(())
    }

    async fn get_playername(&self, uuid: &str) -> Option<String> {
        let uuid_map = self.uuid_to_playername.read().await;
        let result = uuid_map.get(uuid).cloned();

        debug!(
            uuid = %uuid,
            playername = ?result,
            "UUID to playername lookup"
        );

        result
    }

    async fn remove_player(&self, uuid: &str) -> bool {
        let mut uuid_map = self.uuid_to_playername.write().await;

        if let Some(playername) = uuid_map.remove(uuid) {
            info!(
                uuid = %uuid,
                playername = %playername,
                "Removed player mapping"
            );

            true
        } else {
            warn!(uuid = %uuid, "Attempted to remove non-existent player mapping");
            false
        }
    }

    async fn get_all_mappings(&self) -> Vec<(String, String)> {
        let uuid_map = self.uuid_to_playername.read().await;
        uuid_map
            .iter()
            .map(|(uuid, playername)| (uuid.clone(), playername.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_player_success() {
        let service = InMemoryPlayerMappingService::new();
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let playername = "test-player";

        let result = service
            .register_player(uuid.to_string(), playername.to_string())
            .await;
        assert!(result.is_ok());

        assert_eq!(
            service.get_playername(uuid).await,
            Some(playername.to_string())
        );
    }

    #[tokio::test]
    async fn test_register_player_duplicate_uuid() {
        let service = InMemoryPlayerMappingService::new();
        let uuid = "550e8400-e29b-41d4-a716-446655440000";

        service
            .register_player(uuid.to_string(), "player1".to_string())
            .await
            .unwrap();

        let result = service
            .register_player(uuid.to_string(), "player2".to_string())
            .await;
        assert!(matches!(
            result,
            Err(MappingError::UuidAlreadyExists { .. })
        ));
    }

    #[tokio::test]
    async fn test_register_player_duplicate_playername_allowed() {
        let service = InMemoryPlayerMappingService::new();
        let playername = "test-player";

        // Same playername with different UUIDs should be allowed
        let result1 = service
            .register_player(
                "550e8400-e29b-41d4-a716-446655440000".to_string(),
                playername.to_string(),
            )
            .await;
        assert!(result1.is_ok());

        let result2 = service
            .register_player(
                "550e8400-e29b-41d4-a716-446655440001".to_string(),
                playername.to_string(),
            )
            .await;
        assert!(result2.is_ok()); // Should succeed since playername uniqueness is not enforced
    }

    #[tokio::test]
    async fn test_register_player_invalid_uuid() {
        let service = InMemoryPlayerMappingService::new();

        let result = service
            .register_player("invalid-uuid".to_string(), "player".to_string())
            .await;
        assert!(matches!(result, Err(MappingError::InvalidUuid { .. })));
    }

    #[tokio::test]
    async fn test_register_bot_uuid() {
        let service = InMemoryPlayerMappingService::new();
        let bot_uuid = format!("bot-{}", Uuid::new_v4());

        let result = service
            .register_player(bot_uuid.clone(), "bot-player".to_string())
            .await;

        assert!(result.is_ok());
        assert_eq!(
            service.get_playername(&bot_uuid).await,
            Some("bot-player".to_string())
        );
    }

    #[tokio::test]
    async fn test_remove_player() {
        let service = InMemoryPlayerMappingService::new();
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let playername = "test-player";

        service
            .register_player(uuid.to_string(), playername.to_string())
            .await
            .unwrap();

        let removed = service.remove_player(uuid).await;
        assert!(removed);

        assert_eq!(service.get_playername(uuid).await, None);
    }

    #[tokio::test]
    async fn test_remove_nonexistent_player() {
        let service = InMemoryPlayerMappingService::new();

        let removed = service
            .remove_player("550e8400-e29b-41d4-a716-446655440000")
            .await;
        assert!(!removed);
    }

    #[tokio::test]
    async fn test_get_all_mappings() {
        let service = InMemoryPlayerMappingService::new();

        service
            .register_player(
                "550e8400-e29b-41d4-a716-446655440000".to_string(),
                "player1".to_string(),
            )
            .await
            .unwrap();
        service
            .register_player(
                "550e8400-e29b-41d4-a716-446655440001".to_string(),
                "player2".to_string(),
            )
            .await
            .unwrap();

        let mappings = service.get_all_mappings().await;
        assert_eq!(mappings.len(), 2);

        // Verify both mappings exist
        assert!(mappings
            .iter()
            .any(|(uuid, _)| uuid == "550e8400-e29b-41d4-a716-446655440000"));
        assert!(mappings
            .iter()
            .any(|(uuid, _)| uuid == "550e8400-e29b-41d4-a716-446655440001"));
    }

    #[test]
    fn test_generate_uuid() {
        let uuid1 = InMemoryPlayerMappingService::generate_uuid();
        let uuid2 = InMemoryPlayerMappingService::generate_uuid();

        // Should generate valid UUIDs
        assert!(Uuid::parse_str(&uuid1).is_ok());
        assert!(Uuid::parse_str(&uuid2).is_ok());

        // Should be unique
        assert_ne!(uuid1, uuid2);
    }
}
