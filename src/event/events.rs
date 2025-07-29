use serde::{Deserialize, Serialize};

/// Room-specific events (delivered only to room subscribers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoomEvent {
    /// A player joined this room
    PlayerJoined { player: String },
    /// A player left this room
    PlayerLeft { player: String },
    /// The host of this room changed
    HostChanged { old_host: String, new_host: String },
    /// A chat message was sent in this room
    ChatMessage { sender: String, content: String },
    /// A player explicitly requested to leave (different from disconnect)
    PlayerLeaveRequested { player: String },
    /// WebSocket connection was established for a player
    PlayerConnected { player: String },
    /// WebSocket connection was lost for a player  
    PlayerDisconnected { player: String },
}
