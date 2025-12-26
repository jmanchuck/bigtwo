use std::sync::Arc;
use tracing::{debug, instrument};

use super::repository::RoomRepository;
use crate::shared::AppError;

/// Service for tracking room activity timestamps
pub struct ActivityTracker {
    room_repository: Arc<dyn RoomRepository + Send + Sync>,
}

impl ActivityTracker {
    /// Creates a new activity tracker with the given room repository
    pub fn new(room_repository: Arc<dyn RoomRepository + Send + Sync>) -> Self {
        Self { room_repository }
    }

    /// Records activity in a room by updating its last_activity_at timestamp
    #[instrument(skip(self))]
    pub async fn record_activity(&self, room_id: &str) -> Result<(), AppError> {
        debug!(room_id = %room_id, "Recording room activity");
        self.room_repository.update_last_activity(room_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room::models::RoomModel;
    use crate::room::repository::InMemoryRoomRepository;

    #[tokio::test]
    async fn test_record_activity_updates_timestamp() {
        let repo = Arc::new(InMemoryRoomRepository::new());
        let tracker = ActivityTracker::new(repo.clone());

        // Create a room
        let room = RoomModel::new("test-host".to_string());
        let room_id = room.id.clone();
        let initial_activity = room.last_activity_at;
        repo.create_room(&room).await.unwrap();

        // Wait a small amount of time to ensure timestamp changes
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Record activity
        tracker.record_activity(&room_id).await.unwrap();

        // Verify timestamp was updated
        let updated_room = repo.get_room(&room_id).await.unwrap().unwrap();
        assert!(
            updated_room.last_activity_at > initial_activity,
            "Last activity timestamp should be updated"
        );
    }

    #[tokio::test]
    async fn test_record_activity_nonexistent_room() {
        let repo = Arc::new(InMemoryRoomRepository::new());
        let tracker = ActivityTracker::new(repo);

        // Try to record activity for nonexistent room
        let result = tracker.record_activity("nonexistent-room").await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_multiple_activity_updates() {
        let repo = Arc::new(InMemoryRoomRepository::new());
        let tracker = ActivityTracker::new(repo.clone());

        // Create a room
        let room = RoomModel::new("test-host".to_string());
        let room_id = room.id.clone();
        repo.create_room(&room).await.unwrap();

        let mut last_timestamp = room.last_activity_at;

        // Record multiple activities
        for _ in 0..5 {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            tracker.record_activity(&room_id).await.unwrap();

            let updated_room = repo.get_room(&room_id).await.unwrap().unwrap();
            assert!(
                updated_room.last_activity_at > last_timestamp,
                "Each activity should update the timestamp"
            );
            last_timestamp = updated_room.last_activity_at;
        }
    }
}
