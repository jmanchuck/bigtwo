use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;
use tracing::{debug, info, instrument, warn};

use super::models::RoomModel;
use crate::shared::AppError;

/// Result of attempting to join a room
#[derive(Debug, Clone)]
pub enum JoinRoomResult {
    /// Successfully joined the room, returns updated room data
    Success(RoomModel),
    /// Room is at capacity (4 players)
    RoomFull,
    /// Room does not exist
    RoomNotFound,
}

/// Result of attempting to leave a room
#[derive(Debug, Clone)]
pub enum LeaveRoomResult {
    /// Successfully left the room, returns updated room data
    Success(RoomModel),
    /// Player was not in the room
    PlayerNotInRoom,
    /// Room does not exist
    RoomNotFound,
    /// Room was deleted because no players left
    RoomDeleted,
}

/// Trait for room repository operations
#[async_trait]
pub trait RoomRepository {
    async fn create_room(&self, room: &RoomModel) -> Result<(), AppError>;
    async fn get_room(&self, room_id: &str) -> Result<Option<RoomModel>, AppError>;
    async fn list_rooms(&self) -> Result<Vec<RoomModel>, AppError>;

    /// Atomically attempts to join a room by checking capacity and incrementing player count
    /// This prevents race conditions when multiple players try to join simultaneously
    async fn try_join_room(
        &self,
        room_id: &str,
        player_name: &str,
    ) -> Result<JoinRoomResult, AppError>;

    /// Atomically attempts to remove a player from a room
    async fn leave_room(
        &self,
        room_id: &str,
        player_name: &str,
    ) -> Result<LeaveRoomResult, AppError>;
}

/// In-memory implementation of RoomRepository for development and testing
pub struct InMemoryRoomRepository {
    rooms: Mutex<HashMap<String, RoomModel>>,
}

impl Default for InMemoryRoomRepository {
    fn default() -> Self {
        Self::new()
    }
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

    #[instrument(skip(self))]
    async fn list_rooms(&self) -> Result<Vec<RoomModel>, AppError> {
        debug!("Listing all rooms in memory");

        let rooms = self.rooms.lock().unwrap();
        let room_list = rooms.values().cloned().collect();

        debug!("Rooms listed successfully in memory");
        Ok(room_list)
    }

    #[instrument(skip(self))]
    async fn try_join_room(
        &self,
        room_id: &str,
        player_name: &str,
    ) -> Result<JoinRoomResult, AppError> {
        debug!(room_id = %room_id, player_name = %player_name, "Attempting to join room atomically");

        let mut rooms = self.rooms.lock().unwrap();

        // Get the room or return RoomNotFound
        let room = match rooms.get_mut(room_id) {
            Some(room) => room,
            None => {
                debug!(room_id = %room_id, "Room not found");
                return Ok(JoinRoomResult::RoomNotFound);
            }
        };

        // Check if room is at capacity
        if room.is_full() {
            debug!(room_id = %room_id, current_count = room.get_player_count(), "Room is full");
            return Ok(JoinRoomResult::RoomFull);
        }

        // Check if player is already in room (prevent duplicates)
        if room.has_player(player_name) {
            debug!(room_id = %room_id, player_name = %player_name, "Player already in room");
            return Ok(JoinRoomResult::Success(room.clone()));
        }

        // Add player to the room
        room.players.push(player_name.to_string());

        // Clone the updated room data to return
        let updated_room = room.clone();

        info!(
            room_id = %room_id,
            player_name = %player_name,
            new_player_count = updated_room.get_player_count(),
            "Player joined room successfully (atomic)"
        );

        Ok(JoinRoomResult::Success(updated_room))
    }

    #[instrument(skip(self))]
    async fn leave_room(
        &self,
        room_id: &str,
        player_name: &str,
    ) -> Result<LeaveRoomResult, AppError> {
        info!(room_id = %room_id, player_name = %player_name, "Attempting to leave room atomically");

        let mut rooms = self.rooms.lock().unwrap();

        // Get the room or return RoomNotFound
        let room = match rooms.get_mut(room_id) {
            Some(room) => room,
            None => {
                info!(room_id = %room_id, "Room not found");
                return Ok(LeaveRoomResult::RoomNotFound);
            }
        };

        // Check if player is in the room
        if !room.has_player(player_name) {
            info!(room_id = %room_id, player_name = %player_name, "Player not in room");
            return Ok(LeaveRoomResult::PlayerNotInRoom);
        }

        // Remove player from the room
        room.players.retain(|p| p != player_name);

        // If room is now empty, delete it
        if room.players.is_empty() {
            info!(room_id = %room_id, "Room is now empty, deleting");
            rooms.remove(room_id);
            return Ok(LeaveRoomResult::RoomDeleted);
        }

        // If the leaving player was the host, assign new host to first remaining player
        if room.host_name == player_name {
            if let Some(new_host) = room.players.first().cloned() {
                info!(
                    room_id = %room_id,
                    old_host = %player_name,
                    new_host = %new_host,
                    "Host left, assigning new host"
                );
                room.host_name = new_host;
            }
        }

        // Clone the updated room data to return
        let updated_room = room.clone();

        info!(
            room_id = %room_id,
            player_name = %player_name,
            new_player_count = updated_room.get_player_count(),
            current_host = %updated_room.host_name,
            "Player left room successfully (atomic)"
        );

        Ok(LeaveRoomResult::Success(updated_room))
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
                players: vec![host_name.to_string()], // Host is first player
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
        assert_eq!(retrieved_room.get_player_count(), 1);
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

    #[tokio::test]
    async fn test_list_rooms_empty() {
        let repo = InMemoryRoomRepository::new();

        let rooms = repo.list_rooms().await.unwrap();
        assert!(rooms.is_empty());
    }

    #[tokio::test]
    async fn test_list_rooms_single() {
        let repo = InMemoryRoomRepository::new();
        let room = create_test_room("test-room", "test-host");

        repo.create_room(&room).await.unwrap();

        let rooms = repo.list_rooms().await.unwrap();
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].id, "test-room");
        assert_eq!(rooms[0].host_name, "test-host");
    }

    #[tokio::test]
    async fn test_list_rooms_multiple() {
        let repo = InMemoryRoomRepository::new();
        let room1 = create_test_room("room-1", "host-1");
        let room2 = create_test_room("room-2", "host-2");
        let room3 = create_test_room("room-3", "host-3");

        // Create all rooms
        repo.create_room(&room1).await.unwrap();
        repo.create_room(&room2).await.unwrap();
        repo.create_room(&room3).await.unwrap();

        // List all rooms
        let rooms = repo.list_rooms().await.unwrap();
        assert_eq!(rooms.len(), 3);

        // Verify all rooms are present (order may vary due to HashMap)
        let room_ids: std::collections::HashSet<String> =
            rooms.iter().map(|r| r.id.clone()).collect();
        assert!(room_ids.contains("room-1"));
        assert!(room_ids.contains("room-2"));
        assert!(room_ids.contains("room-3"));

        // Verify host names are correct
        let room_hosts: std::collections::HashMap<String, String> = rooms
            .iter()
            .map(|r| (r.id.clone(), r.host_name.clone()))
            .collect();
        assert_eq!(room_hosts.get("room-1"), Some(&"host-1".to_string()));
        assert_eq!(room_hosts.get("room-2"), Some(&"host-2".to_string()));
        assert_eq!(room_hosts.get("room-3"), Some(&"host-3".to_string()));
    }
}
