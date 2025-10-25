use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct GameResult {
    pub room_id: String,
    #[allow(dead_code)] // Metadata for game tracking
    pub game_number: u32,
    pub winner_uuid: String,
    pub players: Vec<PlayerGameResult>,
    #[allow(dead_code)] // Metadata for future analytics
    pub completed_at: DateTime<Utc>,
    #[allow(dead_code)] // Metadata for filtering bot games
    pub had_bots: bool,
}

#[derive(Debug, Clone)]
pub struct PlayerGameResult {
    pub uuid: String,
    #[allow(dead_code)] // Used in score calculations
    pub cards_remaining: u8,
    #[allow(dead_code)] // Score before multipliers
    pub raw_score: i32,
    pub final_score: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RoomStats {
    pub room_id: String,
    pub games_played: u32,
    pub player_stats: HashMap<String, PlayerStats>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerStats {
    pub uuid: String,
    pub games_played: u32,
    pub wins: u32,
    pub total_score: i32,
    pub current_win_streak: u32,
    pub best_win_streak: u32,
}

#[derive(Debug, Clone)]
pub enum CollectedData {
    CardsRemaining { player_uuid: String, count: u8 },
    #[allow(dead_code)] // Enum field used via pattern matching
    WinLoss { player_uuid: String, won: bool },
}

impl CollectedData {
    #[allow(dead_code)] // Public API for accessing player UUID
    pub fn player_uuid(&self) -> &str {
        match self {
            CollectedData::CardsRemaining { player_uuid, .. } => player_uuid,
            CollectedData::WinLoss { player_uuid, .. } => player_uuid,
        }
    }
}
