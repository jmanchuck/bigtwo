use async_trait::async_trait;
use std::sync::Arc;
use tracing::debug;

use crate::{
    event::{RoomEvent, RoomEventError, RoomEventHandler},
    room::repository::RoomRepository,
    websockets::{connection_manager::ConnectionManager, messages::WebSocketMessage},
};

/// WebSocket-specific room event handler
///
/// Handles room events by:
/// 1. Querying current room state
/// 2. Converting events to WebSocket messages  
/// 3. Sending to all connected players in the room
pub struct WebSocketRoomSubscriber {
    room_repository: Arc<dyn RoomRepository + Send + Sync>,
    connection_manager: Arc<dyn ConnectionManager>,
}

impl WebSocketRoomSubscriber {
    pub fn new(
        room_repository: Arc<dyn RoomRepository + Send + Sync>,
        connection_manager: Arc<dyn ConnectionManager>,
    ) -> Self {
        Self {
            room_repository,
            connection_manager,
        }
    }
}

#[async_trait]
impl RoomEventHandler for WebSocketRoomSubscriber {
    async fn handle_room_event(
        &self,
        room_id: &str,
        event: RoomEvent,
    ) -> Result<(), RoomEventError> {
        debug!(
            room_id = %room_id,
            event = ?event,
            "Handling room event for WebSocket connections"
        );

        match event {
            RoomEvent::PlayerJoined { player } => {
                // Query current room state for accurate player list
                let room = self
                    .room_repository
                    .get_room(room_id)
                    .await
                    .map_err(|e| {
                        RoomEventError::HandlerError(format!("Failed to get room: {}", e))
                    })?
                    .ok_or_else(|| RoomEventError::RoomNotFound(room_id.to_string()))?;

                // Create WebSocket message for players list using the correct helper function
                let ws_message = WebSocketMessage::players_list(room.players.clone());

                let message_json = serde_json::to_string(&ws_message).map_err(|e| {
                    RoomEventError::HandlerError(format!("Failed to serialize message: {}", e))
                })?;

                // Send to all players in the room
                for player_name in &room.players {
                    self.connection_manager
                        .send_to_player(player_name, &message_json)
                        .await;
                }

                debug!(
                    room_id = %room_id,
                    players_notified = room.players.len(),
                    message = %message_json,
                    "Player joined notification sent to all room players"
                );

                Ok(())
            }
            RoomEvent::PlayerLeft { player } => {
                // Query current room state for accurate player list
                let room = self.room_repository.get_room(room_id).await.map_err(|e| {
                    RoomEventError::HandlerError(format!("Failed to get room: {}", e))
                })?;

                // If room was deleted (no players left), no need to notify anyone
                let room = match room {
                    Some(room) => room,
                    None => {
                        debug!(room_id = %room_id, "Room was deleted, no notifications needed");
                        return Ok(());
                    }
                };

                // Create WebSocket message for players list using the correct helper function
                let ws_message = WebSocketMessage::players_list(room.players.clone());

                let message_json = serde_json::to_string(&ws_message).map_err(|e| {
                    RoomEventError::HandlerError(format!("Failed to serialize message: {}", e))
                })?;

                // Send to all remaining players in the room
                for player_name in &room.players {
                    self.connection_manager
                        .send_to_player(player_name, &message_json)
                        .await;
                }

                debug!(
                    room_id = %room_id,
                    players_notified = room.players.len(),
                    message = %message_json,
                    "Player left notification sent to all remaining room players"
                );

                Ok(())
            }
        }
    }

    fn handler_name(&self) -> &'static str {
        "WebSocketRoomSubscriber"
    }
}
