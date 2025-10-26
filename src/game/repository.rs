use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::game::core::{Game, GameError};

pub struct GameRepository {
    /// A mapping from room ID to game
    games: Arc<RwLock<HashMap<String, Game>>>,
}

impl Default for GameRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl GameRepository {
    pub fn new() -> Self {
        Self {
            games: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_game(
        &self,
        room_id: &str,
        player_data: &[(String, String)],
    ) -> Result<(), GameError> {
        let mut games = self.games.write().await;
        let game = Game::new_game(room_id.to_string(), player_data)?;
        games.insert(room_id.to_string(), game);
        Ok(())
    }

    pub async fn update_game(&self, room_id: &str, game: Game) -> Result<(), GameError> {
        let mut games = self.games.write().await;
        games.insert(room_id.to_string(), game);
        Ok(())
    }

    pub async fn get_game(&self, room_id: &str) -> Option<Game> {
        let games = self.games.read().await;
        games.get(room_id).cloned()
    }

    pub async fn remove_game(&self, room_id: &str) -> Option<Game> {
        let mut games = self.games.write().await;
        games.remove(room_id)
    }
}
