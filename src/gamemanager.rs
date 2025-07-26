use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    event::{EventError, EventHandler, GameEvent},
    game::Game,
    lobby::{Lobby, LobbyError},
};

enum RoomState {
    Lobby(Lobby),
    InGame(Game),
    Completed(Game),
}

pub struct GameManager {
    rooms: Arc<RwLock<HashMap<String, RoomState>>>,
}

impl GameManager {
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_lobby(&self, room_id: String, host: String) -> Result<(), LobbyError> {
        let lobby = Lobby::new(host);
        let mut rooms = self.rooms.write().await;
        rooms.insert(room_id, RoomState::Lobby(lobby));
        Ok(())
    }
}

#[async_trait]
impl EventHandler for GameManager {
    async fn handle(&self, event: &GameEvent) -> Result<(), EventError> {
        match event {
            GameEvent::LobbyCreated { room_id, host } => self
                .create_lobby(room_id.clone(), host.clone())
                .await
                .map_err(|e| EventError::non_retryable(format!("Failed to create lobby: {}", e))),
            _ => Ok(()),
        }
    }

    fn name(&self) -> &'static str {
        "GameManager"
    }
}
