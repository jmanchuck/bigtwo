use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Database model for rooms table
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RoomModel {
    pub id: String,                // Random pet name generated ID
    pub host_uuid: Option<String>, // UUID of room host
    pub status: String,            // "ONLINE" or "OFFLINE"
    pub player_uuids: Vec<String>, // List of player UUIDs in this room (for internal use)
}

impl RoomModel {
    /// Creates a new room model with generated ID
    pub fn new(host_uuid: String) -> Self {
        let room_id = petname::Petnames::default().generate_one(2, "");

        Self {
            id: room_id,
            host_uuid: Some(host_uuid),
            status: "ONLINE".to_string(),
            player_uuids: vec![], // Host UUID will be added when they join
        }
    }

    /// Get the current number of players
    pub fn get_player_count(&self) -> i32 {
        self.player_uuids.len() as i32
    }

    /// Check if room is at capacity (4 players)
    pub fn is_full(&self) -> bool {
        self.player_uuids.len() >= 4
    }

    /// Check if a player is in this room (by UUID)
    pub fn has_player(&self, player_uuid: &str) -> bool {
        self.player_uuids.contains(&player_uuid.to_string())
    }

    /// Get all player UUIDs in the room
    pub fn get_player_uuids(&self) -> &Vec<String> {
        &self.player_uuids
    }

    /// Add a player to the room (both username and UUID)
    pub fn add_player(&mut self, player_uuid: String) {
        if !self.has_player(&player_uuid) {
            self.player_uuids.push(player_uuid);
        }
    }

    /// Remove a player from the room (both username and UUID)
    pub fn remove_player(&mut self, player_uuid: &str) {
        self.player_uuids.retain(|p| p != player_uuid);
    }
}
