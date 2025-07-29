use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Database model for rooms table
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RoomModel {
    pub id: String,           // Random pet name generated ID
    pub host_name: String,    // Username of room host
    pub status: String,       // "ONLINE" or "OFFLINE"
    pub players: Vec<String>, // List of player usernames in this room
}

impl RoomModel {
    /// Creates a new room model with generated ID
    pub fn new(host_name: String) -> Self {
        let room_id = petname::Petnames::default().generate_one(2, "");

        Self {
            id: room_id,
            host_name: host_name.clone(),
            status: "ONLINE".to_string(),
            players: vec![], // Host must join like everyone else
        }
    }

    /// Get the current number of players
    pub fn get_player_count(&self) -> i32 {
        self.players.len() as i32
    }

    /// Check if room is at capacity (4 players)
    pub fn is_full(&self) -> bool {
        self.players.len() >= 4
    }

    /// Check if a player is in this room
    pub fn has_player(&self, username: &str) -> bool {
        self.players.contains(&username.to_string())
    }
}
