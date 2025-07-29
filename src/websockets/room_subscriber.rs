use async_trait::async_trait;
use std::sync::Arc;
use tracing::debug;

use crate::{
    event::{RoomEvent, RoomEventError, RoomEventHandler},
    room::repository::LeaveRoomResult,
    room::repository::RoomRepository,
    room::service::RoomService,
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
    event_bus: crate::event::EventBus,
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
            RoomEvent::PlayerJoined { player: _ } => self.handle_player_joined(room_id).await,
            RoomEvent::PlayerLeft { player } => self.handle_player_left(room_id, &player).await,
            RoomEvent::HostChanged { old_host, new_host } => {
                self.handle_host_changed(room_id, &old_host, &new_host)
                    .await
            }
            RoomEvent::ChatMessage { sender, content } => {
                self.handle_chat_message(room_id, &sender, &content).await
            }
            RoomEvent::PlayerLeaveRequested { player } => {
                self.handle_leave_request(room_id, &player).await
            }
            RoomEvent::PlayerDisconnected { player } => {
                self.handle_leave_request(room_id, &player).await
            }
            _ => {
                debug!(
                    room_id = %room_id,
                    event = ?event,
                    "Unhandled event type in WebSocketRoomSubscriber"
                );
                Ok(())
            }
        }
    }

    fn handler_name(&self) -> &'static str {
        "WebSocketRoomSubscriber"
    }
}

impl WebSocketRoomSubscriber {
    pub fn new(
        room_repository: Arc<dyn RoomRepository + Send + Sync>,
        connection_manager: Arc<dyn ConnectionManager>,
        event_bus: crate::event::EventBus,
    ) -> Self {
        Self {
            room_repository,
            connection_manager,
            event_bus,
        }
    }

    async fn handle_player_joined(&self, room_id: &str) -> Result<(), RoomEventError> {
        debug!(room_id = %room_id, "Handling player joined event");

        // Query current room state for accurate player list
        let room = self
            .room_repository
            .get_room(room_id)
            .await
            .map_err(|e| RoomEventError::HandlerError(format!("Failed to get room: {}", e)))?
            .ok_or_else(|| RoomEventError::RoomNotFound(room_id.to_string()))?;

        // Create WebSocket message for players list
        let ws_message = WebSocketMessage::players_list(room.players.clone());
        let message_json = serde_json::to_string(&ws_message).map_err(|e| {
            RoomEventError::HandlerError(format!("Failed to serialize PLAYERS_LIST message: {}", e))
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
            "Player joined notification sent to all room players"
        );

        Ok(())
    }

    async fn handle_player_left(
        &self,
        room_id: &str,
        player_name: &str,
    ) -> Result<(), RoomEventError> {
        debug!(
            room_id = %room_id,
            player = %player_name,
            "Handling player left event"
        );

        // Query current room state for accurate player list
        let room = self
            .room_repository
            .get_room(room_id)
            .await
            .map_err(|e| RoomEventError::HandlerError(format!("Failed to get room: {}", e)))?;

        // If room was deleted (no players left), no need to notify anyone
        let room = match room {
            Some(room) => room,
            None => {
                debug!(room_id = %room_id, "Room was deleted, no notifications needed");
                return Ok(());
            }
        };

        // Send LEAVE message to notify about the specific player who left
        let leave_message = WebSocketMessage::leave(player_name.to_string());
        let leave_json = serde_json::to_string(&leave_message).map_err(|e| {
            RoomEventError::HandlerError(format!("Failed to serialize LEAVE message: {}", e))
        })?;

        // Send LEAVE notification to all remaining players in the room
        for player in &room.players {
            self.connection_manager
                .send_to_player(player, &leave_json)
                .await;
        }

        // Create WebSocket message for updated players list
        let players_list_message = WebSocketMessage::players_list(room.players.clone());
        let players_list_json = serde_json::to_string(&players_list_message).map_err(|e| {
            RoomEventError::HandlerError(format!("Failed to serialize PLAYERS_LIST message: {}", e))
        })?;

        // Send updated players list to all remaining players in the room
        for player in &room.players {
            self.connection_manager
                .send_to_player(player, &players_list_json)
                .await;
        }

        debug!(
            room_id = %room_id,
            player_left = %player_name,
            players_notified = room.players.len(),
            "Player left notifications sent to all remaining room players"
        );

        Ok(())
    }

    async fn handle_host_changed(
        &self,
        room_id: &str,
        old_host: &str,
        new_host: &str,
    ) -> Result<(), RoomEventError> {
        debug!(
            room_id = %room_id,
            old_host = %old_host,
            new_host = %new_host,
            "Handling host changed event"
        );

        // Query current room state
        let room = self
            .room_repository
            .get_room(room_id)
            .await
            .map_err(|e| RoomEventError::HandlerError(format!("Failed to get room: {}", e)))?;

        let room = match room {
            Some(room) => room,
            None => {
                debug!(room_id = %room_id, "Room was deleted, no host change notifications needed");
                return Ok(());
            }
        };

        // Create WebSocket message for host change
        let host_change_message = WebSocketMessage::host_change(new_host.to_string());
        let message_json = serde_json::to_string(&host_change_message).map_err(|e| {
            RoomEventError::HandlerError(format!("Failed to serialize HOST_CHANGE message: {}", e))
        })?;

        // Send to all players in the room
        for player_name in &room.players {
            self.connection_manager
                .send_to_player(player_name, &message_json)
                .await;
        }

        debug!(
            room_id = %room_id,
            old_host = %old_host,
            new_host = %new_host,
            players_notified = room.players.len(),
            "Host change notification sent to all room players"
        );

        Ok(())
    }

    async fn handle_chat_message(
        &self,
        room_id: &str,
        sender: &str,
        content: &str,
    ) -> Result<(), RoomEventError> {
        debug!(
            room_id = %room_id,
            sender = %sender,
            "Handling chat message event"
        );

        // Get current room state to find all players
        let room = self
            .room_repository
            .get_room(room_id)
            .await
            .map_err(|e| RoomEventError::HandlerError(format!("Failed to get room: {}", e)))?;

        let room = match room {
            Some(room) => room,
            None => {
                debug!(room_id = %room_id, "Room was deleted, no chat notifications needed");
                return Ok(());
            }
        };

        // Create chat message to broadcast
        let chat_message = WebSocketMessage::chat(sender.to_string(), content.to_string());
        let message_json = serde_json::to_string(&chat_message).map_err(|e| {
            RoomEventError::HandlerError(format!("Failed to serialize CHAT message: {}", e))
        })?;

        // Send to all players in the room
        for player_name in &room.players {
            self.connection_manager
                .send_to_player(player_name, &message_json)
                .await;
        }

        debug!(
            room_id = %room_id,
            sender = %sender,
            players_notified = room.players.len(),
            "Chat message forwarded to all room players"
        );

        Ok(())
    }

    async fn handle_leave_request(
        &self,
        room_id: &str,
        player_name: &str,
    ) -> Result<(), RoomEventError> {
        debug!(
            room_id = %room_id,
            player = %player_name,
            "Processing leave request"
        );

        // Get room state before leaving to detect host changes
        let room_before = self
            .room_repository
            .get_room(room_id)
            .await
            .map_err(|e| RoomEventError::HandlerError(format!("Failed to get room: {}", e)))?;

        let was_host = room_before
            .as_ref()
            .map(|room| room.host_name == player_name)
            .unwrap_or(false);

        // Perform the leave operation using room service
        let room_service = RoomService::new(Arc::clone(&self.room_repository));
        match room_service
            .leave_room(room_id.to_string(), player_name.to_string())
            .await
        {
            Ok(LeaveRoomResult::Success(updated_room)) => {
                // Emit PlayerLeft event
                self.event_bus
                    .emit_to_room(
                        room_id,
                        RoomEvent::PlayerLeft {
                            player: player_name.to_string(),
                        },
                    )
                    .await;

                // If host changed, emit HostChanged event
                if was_host && updated_room.host_name != player_name {
                    self.event_bus
                        .emit_to_room(
                            room_id,
                            RoomEvent::HostChanged {
                                old_host: player_name.to_string(),
                                new_host: updated_room.host_name.clone(),
                            },
                        )
                        .await;
                }

                debug!(
                    room_id = %room_id,
                    player = %player_name,
                    "Leave request processed successfully"
                );
            }
            Ok(LeaveRoomResult::RoomDeleted) => {
                debug!(
                    room_id = %room_id,
                    player = %player_name,
                    "Room deleted after player left"
                );
            }
            Ok(_) => {
                debug!(
                    room_id = %room_id,
                    player = %player_name,
                    "Player was not in room or room not found"
                );
            }
            Err(e) => {
                return Err(RoomEventError::HandlerError(format!(
                    "Failed to process leave: {}",
                    e
                )));
            }
        }

        Ok(())
    }
}
