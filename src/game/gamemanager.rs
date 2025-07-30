use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::game::game::{Game, GameError};
use crate::shared::AppError;

enum RoomState {
    Lobby,
    InGame(Game),
    Completed(Game),
}

pub struct GameManager {
    /// A mapping from room ID to game
    games: Arc<RwLock<HashMap<String, Game>>>,
}

impl GameManager {
    pub fn new() -> Self {
        Self {
            games: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_game(&self, room_id: &str, players: &[String]) -> Result<(), GameError> {
        let mut games = self.games.write().await;
        let game = Game::new_game(room_id.to_string(), players)?;
        games.insert(room_id.to_string(), game);
        Ok(())
    }

    pub async fn get_game(&self, room_id: &str) -> Option<Game> {
        let games = self.games.read().await;
        games.get(room_id).cloned()
    }

    pub async fn delete_game(&self, room_id: &str) -> Result<(), AppError> {
        let mut games = self.games.write().await;
        games.remove(room_id);
        Ok(())
    }

    pub async fn has_game(&self, room_id: &str) -> bool {
        let games = self.games.read().await;
        games.contains_key(room_id)
    }
}
