use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Message types for WebSocket communication
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    GameWon,
    GameReset,
}

/// Metadata for WebSocket messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessageMeta {
    pub timestamp: DateTime<Utc>,
    pub player_uuid: Option<String>,
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
    pub sender_uuid: String,
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
    /// Player UUIDs currently in the room
    pub players: Vec<String>,
    /// Mapping from UUID to display name for UI resolution
    pub mapping: std::collections::HashMap<String, String>,
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
pub struct ErrorPayload {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStartedPayload {
    pub current_turn: String,
    pub cards: Vec<String>, // Player's hand
    pub player_list: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_played_cards: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_played_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnChangePayload {
    pub player: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameWonPayload {
    pub winner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameResetPayload {
    // Empty payload - just signals that game should reset to lobby
}

/// Helper functions for creating messages
impl WebSocketMessage {
    pub fn new(message_type: MessageType, payload: serde_json::Value) -> Self {
        Self {
            message_type,
            payload,
            meta: Some(WebSocketMessageMeta {
                timestamp: Utc::now(),
                player_uuid: None,
            }),
        }
    }

    /// Create a PLAYERS_LIST message
    pub fn players_list(
        players: Vec<String>,
        mapping: std::collections::HashMap<String, String>,
    ) -> Self {
        let payload = PlayersListPayload { players, mapping };
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
        last_played_cards: Option<Vec<String>>,
        last_played_by: Option<String>,
    ) -> Self {
        let payload = GameStartedPayload {
            current_turn,
            cards,
            player_list,
            last_played_cards,
            last_played_by,
        };
        Self::new(
            MessageType::GameStarted,
            serde_json::to_value(payload).unwrap(),
        )
    }

    /// Create a MOVE_PLAYED message
    pub fn move_played(player: String, cards: Vec<String>) -> Self {
        let payload = MovePlayedPayload { player, cards };
        Self::new(
            MessageType::MovePlayed,
            serde_json::to_value(payload).unwrap(),
        )
    }

    /// Create a CHAT message
    pub fn chat(sender_uuid: String, content: String) -> Self {
        let payload = ChatPayload {
            sender_uuid,
            content,
        };
        Self::new(MessageType::Chat, serde_json::to_value(payload).unwrap())
    }

    /// Create a LEAVE message
    pub fn leave(player: String) -> Self {
        let payload = LeavePayload { player };
        Self::new(MessageType::Leave, serde_json::to_value(payload).unwrap())
    }

    /// Create a TURN_CHANGE message
    pub fn turn_change(player: String) -> Self {
        let payload = TurnChangePayload { player };
        Self::new(
            MessageType::TurnChange,
            serde_json::to_value(payload).unwrap(),
        )
    }

    /// Create a GAME_WON message
    pub fn game_won(winner: String) -> Self {
        let payload = GameWonPayload { winner };
        Self::new(MessageType::GameWon, serde_json::to_value(payload).unwrap())
    }

    /// Create a GAME_RESET message
    pub fn game_reset() -> Self {
        let payload = GameResetPayload {};
        Self::new(
            MessageType::GameReset,
            serde_json::to_value(payload).unwrap(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_constructors_and_serialization() {
        // players_list
        let mut map = std::collections::HashMap::new();
        map.insert("u1".to_string(), "Alice".to_string());
        let m = WebSocketMessage::players_list(vec!["u1".to_string()], map.clone());
        assert!(matches!(m.message_type, MessageType::PlayersList));
        let s = serde_json::to_string(&m).unwrap();
        let back: WebSocketMessage = serde_json::from_str(&s).unwrap();
        assert!(matches!(back.message_type, MessageType::PlayersList));

        // error
        let e = WebSocketMessage::error("oops".to_string());
        assert!(matches!(e.message_type, MessageType::Error));

        // host_change
        let h = WebSocketMessage::host_change("u1".to_string());
        assert!(matches!(h.message_type, MessageType::HostChange));

        // game_started
        let gs = WebSocketMessage::game_started(
            "u1".to_string(),
            vec!["3D".to_string()],
            vec!["u1".to_string()],
            Some(vec!["4H".to_string()]),
            Some("u2".to_string()),
        );
        assert!(matches!(gs.message_type, MessageType::GameStarted));

        // move_played
        let mp = WebSocketMessage::move_played("u1".to_string(), vec!["3D".to_string()]);
        assert!(matches!(mp.message_type, MessageType::MovePlayed));

        // chat
        let c = WebSocketMessage::chat("u1".to_string(), "hi".to_string());
        assert!(matches!(c.message_type, MessageType::Chat));

        // leave
        let l = WebSocketMessage::leave("u1".to_string());
        assert!(matches!(l.message_type, MessageType::Leave));

        // turn_change
        let t = WebSocketMessage::turn_change("u2".to_string());
        assert!(matches!(t.message_type, MessageType::TurnChange));

        // game_won
        let gw = WebSocketMessage::game_won("u3".to_string());
        assert!(matches!(gw.message_type, MessageType::GameWon));

        // game_reset
        let gr = WebSocketMessage::game_reset();
        assert!(matches!(gr.message_type, MessageType::GameReset));
    }
}
