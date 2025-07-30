use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Message types for WebSocket communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MessageType {
    // Client -> Server
    Chat,
    Move,
    Leave,
    StartGame,

    // Server -> Client
    PlayersList,
    HostChange,
    MovePlayed,
    TurnChange,
    Error,
    GameStarted,
}

/// Metadata for WebSocket messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessageMeta {
    pub timestamp: DateTime<Utc>,
    pub player_id: Option<String>,
}

/// Base structure for WebSocket messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub payload: serde_json::Value,
    pub meta: Option<WebSocketMessageMeta>,
}

/// Client-to-Server message payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatPayload {
    pub sender: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovePayload {
    pub cards: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeavePayload {
    pub player: String,
}

/// Server-to-Client message payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayersListPayload {
    pub players: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostChangePayload {
    pub host: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovePlayedPayload {
    pub player: String,
    pub cards: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnChangePayload {
    pub player: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStartedPayload {
    pub current_turn: String,
    pub cards: Vec<String>, // Player's hand
    pub player_list: Vec<String>,
}

/// Helper functions for creating messages
impl WebSocketMessage {
    pub fn new(message_type: MessageType, payload: serde_json::Value) -> Self {
        Self {
            message_type,
            payload,
            meta: Some(WebSocketMessageMeta {
                timestamp: Utc::now(),
                player_id: None,
            }),
        }
    }

    /// Create a PLAYERS_LIST message
    pub fn players_list(players: Vec<String>) -> Self {
        let payload = PlayersListPayload { players };
        Self::new(
            MessageType::PlayersList,
            serde_json::to_value(payload).unwrap(),
        )
    }

    /// Create a HOST_CHANGE message
    pub fn host_change(host: String) -> Self {
        let payload = HostChangePayload { host };
        Self::new(
            MessageType::HostChange,
            serde_json::to_value(payload).unwrap(),
        )
    }

    /// Create an ERROR message
    pub fn error(message: String) -> Self {
        let payload = ErrorPayload { message };
        Self::new(MessageType::Error, serde_json::to_value(payload).unwrap())
    }

    /// Create a GAME_STARTED message
    pub fn game_started(
        current_turn: String,
        cards: Vec<String>,
        player_list: Vec<String>,
    ) -> Self {
        let payload = GameStartedPayload {
            current_turn,
            cards,
            player_list,
        };
        Self::new(
            MessageType::GameStarted,
            serde_json::to_value(payload).unwrap(),
        )
    }

    /// Create a CHAT message
    pub fn chat(sender: String, content: String) -> Self {
        let payload = ChatPayload { sender, content };
        Self::new(MessageType::Chat, serde_json::to_value(payload).unwrap())
    }

    /// Create a LEAVE message
    pub fn leave(player: String) -> Self {
        let payload = LeavePayload { player };
        Self::new(MessageType::Leave, serde_json::to_value(payload).unwrap())
    }
}
