use std::sync::Arc;

use async_trait::async_trait;
use tracing::info;

use crate::{
    event::{EventBus, RoomEvent, RoomEventError, RoomEventHandler},
    game::gamemanager::GameManager,
};

pub struct GameEventRoomSubscriber {
    game_manager: Arc<GameManager>,
    event_bus: EventBus,
}

#[async_trait]
impl RoomEventHandler for GameEventRoomSubscriber {
    async fn handle_room_event(
        &self,
        room_id: &str,
        event: RoomEvent,
    ) -> Result<(), RoomEventError> {
        info!(
            room_id = %room_id,
            event = ?event,
            "Handling game event for WebSocket connections"
        );

        match event {
            RoomEvent::CreateGame { players } => {
                self.handle_create_game(room_id, &players).await?;
            }
            _ => {}
        }

        Ok(())
    }

    fn handler_name(&self) -> &'static str {
        "GameEventRoomSubscriber"
    }
}

impl GameEventRoomSubscriber {
    pub fn new(game_manager: Arc<GameManager>, event_bus: EventBus) -> Self {
        Self {
            game_manager,
            event_bus,
        }
    }

    async fn handle_create_game(
        &self,
        room_id: &str,
        players: &[String],
    ) -> Result<(), RoomEventError> {
        info!(room_id = %room_id, "Starting Game");

        self.game_manager
            .create_game(room_id, players)
            .await
            .map_err(|e| RoomEventError::HandlerError(format!("Failed to create game: {}", e)))?;

        let game =
            self.game_manager
                .get_game(room_id)
                .await
                .ok_or(RoomEventError::HandlerError(format!(
                    "Game not found for room: {}",
                    room_id
                )))?;

        let game_message = RoomEvent::StartGame { game: game.clone() };

        self.event_bus.emit_to_room(room_id, game_message).await;

        Ok(())
    }
}
