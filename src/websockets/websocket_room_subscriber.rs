use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::{
    event::{EventBus, RoomEvent, RoomEventError, RoomEventHandler, RoomSubscription},
    game::{Card, Game, GameEventRoomSubscriber, GameManager},
    room::{
        repository::{LeaveRoomResult, RoomRepository},
        service::RoomService,
    },
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
    game_manager: Arc<GameManager>,
    event_bus: EventBus,
}

#[async_trait]
impl RoomEventHandler for WebSocketRoomSubscriber {
    async fn handle_room_event(
        &self,
        room_id: &str,
        event: RoomEvent,
    ) -> Result<(), RoomEventError> {
        info!(
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
            RoomEvent::StartGame { game } => self.handle_start_game(room_id, game).await,
            RoomEvent::TryStartGame { host } => self.handle_try_start_game(room_id, &host).await,
            RoomEvent::MovePlayed {
                player,
                cards,
                game,
            } => {
                self.handle_move_played(room_id, &player, &cards, game)
                    .await
            }
            RoomEvent::TurnChanged { player } => self.handle_turn_changed(room_id, &player).await,
            _ => {
                info!(
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
        game_manager: Arc<GameManager>,
        event_bus: crate::event::EventBus,
    ) -> Self {
        Self {
            room_repository,
            connection_manager,
            game_manager,
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
        info!(
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
                warn!(room_id = %room_id, "Room was deleted, no host change notifications needed");
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

        info!(
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
        info!(
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
                warn!(room_id = %room_id, "Room was deleted, no chat notifications needed");
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

        info!(
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
        info!(
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

                info!(
                    room_id = %room_id,
                    player = %player_name,
                    "Leave request processed successfully"
                );
            }
            Ok(LeaveRoomResult::RoomDeleted) => {
                info!(
                    room_id = %room_id,
                    player = %player_name,
                    "Room deleted after player left"
                );
            }
            Ok(_) => {
                info!(
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

    async fn handle_start_game(&self, room_id: &str, game: Game) -> Result<(), RoomEventError> {
        info!(room_id = %room_id, "Starting game");

        let current_player_turn = game.current_player_turn();

        for player in game.players() {
            let player_message = WebSocketMessage::game_started(
                current_player_turn.clone(),
                player.cards.iter().map(|card| card.to_string()).collect(),
                game.players()
                    .iter()
                    .map(|player| player.name.clone())
                    .collect(),
            );

            let message_json = serde_json::to_string(&player_message).map_err(|e| {
                RoomEventError::HandlerError(format!(
                    "Failed to serialize GAME_STARTED message: {}",
                    e
                ))
            })?;

            self.connection_manager
                .send_to_player(&player.name, &message_json)
                .await;
        }

        Ok(())
    }

    async fn handle_try_start_game(&self, room_id: &str, host: &str) -> Result<(), RoomEventError> {
        info!(
            room_id = %room_id,
            host = %host,
            "Handling start game event"
        );

        // Check if the host is the current host
        let room = self
            .room_repository
            .get_room(room_id)
            .await
            .map_err(|e| RoomEventError::HandlerError(format!("Failed to get room: {}", e)))?
            .ok_or(RoomEventError::RoomNotFound(room_id.to_string()))?;

        if room.host_name != host {
            info!(room_id = %room_id, "Host is not the current host, cannot start game");
            return Ok(());
        }

        // Check if the room has 4 players
        if room.players.len() != 4 {
            info!(room_id = %room_id, "Room does not have 4 players, cannot start game");
            return Ok(());
        }

        // Create the GameEventRoomSubscriber
        let game_event_room_subscriber = Arc::new(GameEventRoomSubscriber::new(
            Arc::clone(&self.game_manager),
            self.event_bus.clone(),
        ));

        let game_event_room_subscription = RoomSubscription::new(
            room_id.to_string(),
            game_event_room_subscriber,
            self.event_bus.clone(),
        );

        let _subscription_handle = game_event_room_subscription.start().await;

        self.event_bus
            .emit_to_room(
                room_id,
                RoomEvent::CreateGame {
                    players: room.players.clone(),
                },
            )
            .await;

        Ok(())
    }

    async fn handle_move_played(
        &self,
        room_id: &str,
        player: &str,
        cards: &[Card],
        game: Game,
    ) -> Result<(), RoomEventError> {
        info!(
            room_id = %room_id,
            player = %player,
            cards = ?cards,
            "Handling move played event"
        );

        for game_player in game.players() {
            let player_message = WebSocketMessage::move_played(
                player.to_string(),
                cards.iter().map(|card| card.to_string()).collect(),
            );

            let message_json = serde_json::to_string(&player_message).map_err(|e| {
                RoomEventError::HandlerError(format!(
                    "Failed to serialize MOVE_PLAYED message: {}",
                    e
                ))
            })?;

            self.connection_manager
                .send_to_player(&game_player.name, &message_json)
                .await;
        }

        Ok(())
    }

    async fn handle_turn_changed(&self, room_id: &str, player: &str) -> Result<(), RoomEventError> {
        info!(
            room_id = %room_id,
            player = %player,
            "Handling turn changed event"
        );

        // Get current game to find all players
        let game =
            self.game_manager
                .get_game(room_id)
                .await
                .ok_or(RoomEventError::HandlerError(format!(
                    "Game not found for room: {}",
                    room_id
                )))?;

        // Create turn change message
        let turn_change_message = WebSocketMessage::turn_change(player.to_string());
        let message_json = serde_json::to_string(&turn_change_message).map_err(|e| {
            RoomEventError::HandlerError(format!("Failed to serialize TURN_CHANGE message: {}", e))
        })?;

        // Send to all players in the game
        for game_player in game.players() {
            self.connection_manager
                .send_to_player(&game_player.name, &message_json)
                .await;
        }

        info!(
            room_id = %room_id,
            player = %player,
            players_notified = game.players().len(),
            "Turn change notification sent to all players"
        );

        Ok(())
    }
}
