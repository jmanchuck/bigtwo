use crate::cards::Card;
use serde::{Deserialize, Serialize};

/// Events that can occur in the Big Two game
///
/// Events represent facts about things that have already happened.
/// They are used to communicate state changes between different parts
/// of the system without tight coupling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    // Lobby lifecycle events
    /// A new lobby has been created for a room
    LobbyCreated { room_id: String, host: String },

    /// A player has joined the lobby
    PlayerJoined {
        room_id: String,
        player: String,
        current_players: Vec<String>,
    },

    /// A player has left the lobby or game
    PlayerLeft {
        room_id: String,
        player: String,
        remaining_players: Vec<String>,
    },

    // Game lifecycle events
    /// The game has started (lobby → active game)
    GameStarted {
        room_id: String,
        players: Vec<String>,
    },

    /// A player has played cards
    CardPlayed {
        room_id: String,
        player: String,
        cards: Vec<Card>,
    },

    /// A player has passed their turn
    PlayerPassed { room_id: String, player: String },

    /// The turn has changed to the next player
    TurnChanged {
        room_id: String,
        next_player: String,
        current_turn_index: usize,
    },

    /// The game has been completed
    GameCompleted {
        room_id: String,
        winner: String,
        final_scores: Vec<(String, i32)>, // (player, score)
    },

    // Error events
    /// An invalid move was attempted
    InvalidMove {
        room_id: String,
        player: String,
        reason: String,
    },
}

impl GameEvent {
    /// Get the room_id associated with this event
    /// All events are room-specific in our game
    pub fn room_id(&self) -> &str {
        match self {
            GameEvent::LobbyCreated { room_id, .. } => room_id,
            GameEvent::PlayerJoined { room_id, .. } => room_id,
            GameEvent::PlayerLeft { room_id, .. } => room_id,
            GameEvent::GameStarted { room_id, .. } => room_id,
            GameEvent::CardPlayed { room_id, .. } => room_id,
            GameEvent::PlayerPassed { room_id, .. } => room_id,
            GameEvent::TurnChanged { room_id, .. } => room_id,
            GameEvent::GameCompleted { room_id, .. } => room_id,
            GameEvent::InvalidMove { room_id, .. } => room_id,
        }
    }

    /// Get a human-readable description of the event type
    pub fn event_type(&self) -> &'static str {
        match self {
            GameEvent::LobbyCreated { .. } => "lobby_created",
            GameEvent::PlayerJoined { .. } => "player_joined",
            GameEvent::PlayerLeft { .. } => "player_left",
            GameEvent::GameStarted { .. } => "game_started",
            GameEvent::CardPlayed { .. } => "card_played",
            GameEvent::PlayerPassed { .. } => "player_passed",
            GameEvent::TurnChanged { .. } => "turn_changed",
            GameEvent::GameCompleted { .. } => "game_completed",
            GameEvent::InvalidMove { .. } => "invalid_move",
        }
    }
}
