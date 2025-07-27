use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, error};

use crate::{
    event::{RoomEvent, RoomEventError, RoomEventHandler},
    room::repository::RoomRepository,
    websockets::{
        connection_manager::ConnectionManager,
        messages::{MessageType, WebSocketMessage},
    },
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

                // TODO: Get actual player list from room
                // For now, just notify about the joined player
                let ws_message = WebSocketMessage {
                    message_type: MessageType::PlayersList,
                    payload: serde_json::json!({
                        "type": "player_joined",
                        "player": player,
                        "player_count": room.player_count
                    }),
                    meta: None,
                };

                let message_json = serde_json::to_string(&ws_message).map_err(|e| {
                    RoomEventError::HandlerError(format!("Failed to serialize message: {}", e))
                })?;

                // TODO: Send to all players in room
                // For now, we need room model to track actual player names
                // This is where we'd get room.get_player_names() and iterate

                debug!(
                    room_id = %room_id,
                    message = %message_json,
                    "Converted room event to WebSocket message"
                );

                // Placeholder: Would send to all players when we have player list
                // for player_name in room.get_player_names() {
                //     self.connection_manager.send_to_player(&player_name, &message_json).await;
                // }

                Ok(())
            }
        }
    }

    fn handler_name(&self) -> &'static str {
        "WebSocketRoomSubscriber"
    }
}
