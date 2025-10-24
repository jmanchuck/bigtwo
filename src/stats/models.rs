use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct GameResult {
    pub room_id: String,
    pub game_number: u32,
    pub winner_uuid: String,
    pub players: Vec<PlayerGameResult>,
    pub completed_at: DateTime<Utc>,
    pub had_bots: bool,
}

#[derive(Debug, Clone)]
pub struct PlayerGameResult {
    pub uuid: String,
    pub cards_remaining: u8,
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
    WinLoss { player_uuid: String, won: bool },
}

impl CollectedData {
    pub fn player_uuid(&self) -> &str {
        match self {
            CollectedData::CardsRemaining { player_uuid, .. } => player_uuid,
            CollectedData::WinLoss { player_uuid, .. } => player_uuid,
        }
    }
}
