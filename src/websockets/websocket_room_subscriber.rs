use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;

use crate::{
    event::{RoomEvent, RoomEventError, RoomEventHandler},
    game::GameService,
    room::service::RoomService,
    user::PlayerMappingService,
    websockets::connection_manager::ConnectionManager,
};

use super::event_handlers::{
    ChatEventHandlers, ConnectionEventHandlers, GameEventHandlers, RoomEventHandlers,
};

/// WebSocket-specific room event handler
///
/// Handles room events by delegating to specialized event handlers:
/// - RoomEventHandlers: PlayerJoined, PlayerLeft, HostChanged
/// - ChatEventHandlers: ChatMessage
/// - GameEventHandlers: StartGame, MovePlayed, TurnChanged, GameWon, GameReset
/// - ConnectionEventHandlers: PlayerDisconnected, leave requests
pub struct WebSocketRoomSubscriber {
    room_handlers: RoomEventHandlers,
    chat_handlers: ChatEventHandlers,
    game_handlers: GameEventHandlers,
    connection_handlers: ConnectionEventHandlers,
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
            RoomEvent::PlayerJoined { player: _ } => {
                self.room_handlers.handle_player_joined(room_id).await
            }
            RoomEvent::PlayerLeft { player } => {
                self.room_handlers
                    .handle_player_left(room_id, &player)
                    .await
            }
            RoomEvent::HostChanged { old_host, new_host } => {
                self.room_handlers
                    .handle_host_changed(room_id, &old_host, &new_host)
                    .await
            }
            RoomEvent::ChatMessage { sender, content } => {
                self.chat_handlers
                    .handle_chat_message(room_id, &sender, &content)
                    .await
            }
            RoomEvent::PlayerLeaveRequested { player } => {
                self.connection_handlers
                    .handle_leave_request(room_id, &player)
                    .await
            }
            RoomEvent::PlayerDisconnected { player } => {
                self.connection_handlers
                    .handle_leave_request(room_id, &player)
                    .await
            }
            RoomEvent::StartGame { game } => {
                self.game_handlers.handle_start_game(room_id, game).await
            }
            RoomEvent::TryStartGame { host } => {
                self.game_handlers
                    .handle_try_start_game(room_id, &host)
                    .await
            }
            RoomEvent::MovePlayed {
                player,
                cards,
                game,
            } => {
                self.game_handlers
                    .handle_move_played(room_id, &player, &cards, game)
                    .await
            }
            RoomEvent::TurnChanged { player } => {
                self.game_handlers
                    .handle_turn_changed(room_id, &player)
                    .await
            }
            RoomEvent::GameWon { winner } => {
                self.game_handlers.handle_game_won(room_id, &winner).await
            }
            RoomEvent::GameReset => self.game_handlers.handle_game_reset(room_id).await,
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
        room_service: Arc<RoomService>,
        connection_manager: Arc<dyn ConnectionManager>,
        game_service: Arc<GameService>,
        player_mapping: Arc<dyn PlayerMappingService>,
        event_bus: crate::event::EventBus,
    ) -> Self {
        let room_handlers = RoomEventHandlers::new(
            Arc::clone(&room_service),
            Arc::clone(&connection_manager),
            Arc::clone(&player_mapping),
        );

        let chat_handlers =
            ChatEventHandlers::new(Arc::clone(&room_service), Arc::clone(&connection_manager));

        let game_handlers = GameEventHandlers::new(
            Arc::clone(&room_service),
            Arc::clone(&connection_manager),
            Arc::clone(&game_service),
            event_bus.clone(),
        );

        let connection_handlers = ConnectionEventHandlers::new(
            Arc::clone(&room_service),
            Arc::clone(&connection_manager),
            Arc::clone(&player_mapping),
            event_bus.clone(),
        );

        Self {
            room_handlers,
            chat_handlers,
            game_handlers,
            connection_handlers,
        }
    }
}
