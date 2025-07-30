use async_trait::async_trait;
use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    response::Response,
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::event::EventBus;
use crate::event::RoomEvent;
use crate::shared::{AppError, AppState};
use crate::websockets::messages::{MessageType, WebSocketMessage};

use super::socket::{Connection, MessageHandler};

/// Message handler for receiving WebSocket messages from the client
pub struct WebsocketReceiveHandler {
    event_bus: EventBus,
}

impl WebsocketReceiveHandler {
    pub fn new(event_bus: EventBus) -> Self {
        Self { event_bus }
    }
}

#[async_trait]
impl MessageHandler for WebsocketReceiveHandler {
    async fn handle_message(&self, username: &str, room_id: &str, message: String) {
        info!(
            username = %username,
            room_id = %room_id,
            message = %message,
            "Received message"
        );

        // Parse message and emit appropriate event
        match serde_json::from_str::<WebSocketMessage>(&message) {
            Ok(ws_message) => match ws_message.message_type {
                MessageType::Chat => {
                    if let Some(content) =
                        ws_message.payload.get("content").and_then(|v| v.as_str())
                    {
                        self.event_bus
                            .emit_to_room(
                                room_id,
                                RoomEvent::ChatMessage {
                                    sender: username.to_string(),
                                    content: content.to_string(),
                                },
                            )
                            .await;
                    }
                }
                MessageType::Leave => {
                    self.event_bus
                        .emit_to_room(
                            room_id,
                            RoomEvent::PlayerLeaveRequested {
                                player: username.to_string(),
                            },
                        )
                        .await;
                }
                MessageType::StartGame => {
                    self.event_bus
                        .emit_to_room(
                            room_id,
                            RoomEvent::TryStartGame {
                                host: username.to_string(),
                            },
                        )
                        .await;
                }
                _ => {
                    debug!(
                        message_type = ?ws_message.message_type,
                        "Unhandled message type"
                    );
                }
            },
            Err(e) => {
                warn!(
                    username = %username,
                    room_id = %room_id,
                    error = %e,
                    "Failed to parse WebSocket message"
                );
            }
        }
    }
}

/// Default message handler that just logs incoming messages
pub struct DefaultMessageHandler;

#[async_trait]
impl MessageHandler for DefaultMessageHandler {
    async fn handle_message(&self, username: &str, room_id: &str, message: String) {
        debug!(
            username = %username,
            room_id = %room_id,
            message = %message,
            "Received WebSocket message"
        );

        // TODO: Parse and route messages based on type
        // For example:
        // - Chat messages
        // - Game moves
        // - Leave room requests
        // etc.
    }
}

#[derive(Deserialize)]
pub struct WebSocketQuery {
    token: String,
    player: String,
}

/// WebSocket endpoint that handles authentication via query parameters
/// GET /ws/{room_id}?token=jwt_token&player=username
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Path(room_id): Path<String>,
    Query(query): Query<WebSocketQuery>,
    State(app_state): State<AppState>,
) -> Result<Response, AppError> {
    info!(
        room_id = %room_id,
        username = %query.player,
        "WebSocket connection requested"
    );

    // TODO: Validate JWT token properly when SessionService is public
    // For now, just check that token is not empty
    if query.token.is_empty() {
        warn!("Empty token in WebSocket request");
        return Err(AppError::Unauthorized("Invalid token".to_string()));
    }

    // Verify room exists using repository
    let room_option = app_state.room_repository.get_room(&room_id).await?;
    if room_option.is_none() {
        warn!(
            room_id = %room_id,
            "Room not found, rejecting WebSocket connection"
        );
        return Err(AppError::NotFound("Room not found".to_string()));
    }

    info!(
        room_id = %room_id,
        username = %query.player,
        "Room verified, establishing WebSocket connection"
    );

    let username = query.player.clone();
    Ok(ws.on_upgrade(move |socket| {
        handle_websocket_connection(socket, room_id, username, app_state)
    }))
}

/// Handle the upgraded WebSocket connection
async fn handle_websocket_connection(
    socket: axum::extract::ws::WebSocket,
    room_id: String,
    username: String,
    app_state: AppState,
) {
    info!(
        room_id = %room_id,
        username = %username,
        "WebSocket connection established"
    );

    // Create the outbound channel (app -> client)
    let (outbound_sender, outbound_receiver) = mpsc::unbounded_channel::<String>();

    // Register connection with the connection manager
    app_state
        .connection_manager
        .add_connection(username.clone(), outbound_sender.clone())
        .await;

    // Send initial room state to the newly connected player
    if let Ok(Some(room)) = app_state.room_repository.get_room(&room_id).await {
        let initial_message =
            crate::websockets::messages::WebSocketMessage::players_list(room.players);
        if let Ok(message_json) = serde_json::to_string(&initial_message) {
            let _ = outbound_sender.send(message_json);
            debug!(
                room_id = %room_id,
                username = %username,
                "Sent initial PLAYERS_LIST to newly connected player"
            );
        }
    }

    // Wrap the axum WebSocket in our simple interface
    let socket_wrapper = Box::new(socket);

    // Create message handler (using the new GameRoomMessageHandler)
    let message_handler = Arc::new(WebsocketReceiveHandler::new(app_state.event_bus.clone()));

    // Create and run the connection
    let connection = Connection::new(
        username.clone(),
        room_id.clone(),
        socket_wrapper,
        outbound_receiver,
        message_handler,
    );

    // Run the connection until disconnect
    match connection.run().await {
        Ok(()) => {
            info!(
                room_id = %room_id,
                username = %username,
                "WebSocket connection closed cleanly"
            );
        }
        Err(e) => {
            warn!(
                room_id = %room_id,
                username = %username,
                error = ?e,
                "WebSocket connection error"
            );
        }
    }

    // Cleanup: remove from connection manager and emit disconnect event
    app_state
        .connection_manager
        .remove_connection(&username)
        .await;

    // Emit disconnect event - let the event system handle the rest
    app_state
        .event_bus
        .emit_to_room(
            &room_id,
            crate::event::RoomEvent::PlayerDisconnected {
                player: username.clone(),
            },
        )
        .await;

    info!(
        room_id = %room_id,
        username = %username,
        "WebSocket disconnect event emitted"
    );
}
