use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Database model for rooms table
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RoomModel {
    pub id: String,        // Random pet name generated ID
    pub host_name: String, // Username of room host
    pub status: String,    // "ONLINE" or "OFFLINE"
    pub player_count: i32, // Number of connected players
}

impl RoomModel {
    /// Creates a new room model with generated ID
    pub fn new(host_name: String) -> Self {
        let room_id = petname::Petnames::default().generate_one(2, "");

        Self {
            id: room_id,
            host_name,
            status: "ONLINE".to_string(),
            player_count: 1, // Host counts as first player
        }
    }
}
