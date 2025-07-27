use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
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
