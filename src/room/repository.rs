use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;
use tracing::{debug, instrument, warn};

use super::models::RoomModel;
use crate::shared::AppError;

/// Trait for room repository operations
#[async_trait]
pub trait RoomRepository {
    async fn create_room(&self, room: &RoomModel) -> Result<(), AppError>;
    async fn get_room(&self, room_id: &str) -> Result<Option<RoomModel>, AppError>;
}

/// In-memory implementation of RoomRepository for development and testing
pub struct InMemoryRoomRepository {
    rooms: Mutex<HashMap<String, RoomModel>>,
}

impl InMemoryRoomRepository {
    /// Creates a new empty in-memory repository
    pub fn new() -> Self {
        Self {
            rooms: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl RoomRepository for InMemoryRoomRepository {
    #[instrument(skip(self, room))]
    async fn create_room(&self, room: &RoomModel) -> Result<(), AppError> {
        debug!(room_id = %room.id, host_name = %room.host_name, "Creating room in memory");

        let mut rooms = self.rooms.lock().unwrap();
        if rooms.contains_key(&room.id) {
            warn!(room_id = %room.id, "Room already exists in memory");
            return Err(AppError::DatabaseError("Room already exists".to_string()));
        }
        rooms.insert(room.id.clone(), room.clone());

        debug!(room_id = %room.id, "Room created successfully in memory");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_room(&self, room_id: &str) -> Result<Option<RoomModel>, AppError> {
        debug!(room_id = %room_id, "Fetching room from memory");

        let rooms = self.rooms.lock().unwrap();
        let room = rooms.get(room_id).cloned();

        match &room {
            Some(r) => {
                debug!(room_id = %room_id, host_name = %r.host_name, "Room found in memory")
            }
            None => debug!(room_id = %room_id, "Room not found in memory"),
        }

        Ok(room)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test helper functions for creating test data
    mod helpers {
        use super::*;

        /// Creates a test room with a specific ID and host
        pub fn create_test_room(room_id: &str, host_name: &str) -> RoomModel {
            RoomModel {
                id: room_id.to_string(),
                host_name: host_name.to_string(),
                status: "ONLINE".to_string(),
                player_count: 1,
            }
        }
    }

    use helpers::*;

    #[tokio::test]
    async fn test_create_and_get_room() {
        let repo = InMemoryRoomRepository::new();
        let room = create_test_room("test-room", "test-host");

        // Create room
        repo.create_room(&room).await.unwrap();

        // Get room
        let retrieved = repo.get_room(&room.id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved_room = retrieved.unwrap();
        assert_eq!(retrieved_room.id, room.id);
        assert_eq!(retrieved_room.host_name, room.host_name);
        assert_eq!(retrieved_room.status, "ONLINE");
        assert_eq!(retrieved_room.player_count, 1);
    }

    #[tokio::test]
    async fn test_get_nonexistent_room() {
        let repo = InMemoryRoomRepository::new();

        let result = repo.get_room("nonexistent-room").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_create_duplicate_room() {
        let repo = InMemoryRoomRepository::new();
        let room = create_test_room("test-room", "test-host");

        // Create room
        repo.create_room(&room).await.unwrap();

        // Try to create the same room again
        let result = repo.create_room(&room).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::DatabaseError(_)));
    }

    #[tokio::test]
    async fn test_multiple_rooms() {
        let repo = InMemoryRoomRepository::new();
        let room1 = create_test_room("room-1", "host-1");
        let room2 = create_test_room("room-2", "host-2");
        let room3 = create_test_room("room-3", "host-3");

        // Create all rooms
        repo.create_room(&room1).await.unwrap();
        repo.create_room(&room2).await.unwrap();
        repo.create_room(&room3).await.unwrap();

        // Verify all rooms exist
        let retrieved1 = repo.get_room(&room1.id).await.unwrap();
        assert!(retrieved1.is_some());
        assert_eq!(retrieved1.unwrap().host_name, "host-1");

        let retrieved2 = repo.get_room(&room2.id).await.unwrap();
        assert!(retrieved2.is_some());
        assert_eq!(retrieved2.unwrap().host_name, "host-2");

        let retrieved3 = repo.get_room(&room3.id).await.unwrap();
        assert!(retrieved3.is_some());
        assert_eq!(retrieved3.unwrap().host_name, "host-3");
    }
}
