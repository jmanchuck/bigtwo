use serde::{Deserialize, Serialize};

/// Room-specific events (delivered only to room subscribers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoomEvent {
    /// A player joined this room
    PlayerJoined { player: String },
}
