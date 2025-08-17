use async_trait::async_trait;
use axum::{
    extract::{Path, State, WebSocketUpgrade},
    http::HeaderMap,
    response::Response,
};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::event::EventBus;
use crate::event::RoomEvent;
use crate::game::Card;
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
                MessageType::Move => {
                    if let Some(cards_array) =
                        ws_message.payload.get("cards").and_then(|v| v.as_array())
                    {
                        let card_strings: Vec<String> = cards_array
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();

                        // Convert card strings to Card objects early
                        let cards: Result<Vec<Card>, _> = card_strings
                            .iter()
                            .map(|card_str| Card::from_string(card_str))
                            .collect();

                        match cards {
                            Ok(cards) => {
                                self.event_bus
                                    .emit_to_room(
                                        room_id,
                                        RoomEvent::TryPlayMove {
                                            player: username.to_string(),
                                            cards,
                                        },
                                    )
                                    .await;
                            }
                            Err(e) => {
                                warn!(
                                    username = %username,
                                    room_id = %room_id,
                                    error = %e,
                                    "Invalid card format in move"
                                );
                                // TODO: Send error message back to client
                            }
                        }
                    }
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

/// WebSocket endpoint that handles authentication via Sec-WebSocket-Protocol header
/// GET /ws/{room_id} with JWT token in Sec-WebSocket-Protocol header
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Path(room_id): Path<String>,
    headers: HeaderMap,
    State(app_state): State<AppState>,
) -> Result<Response, AppError> {
    info!(
        room_id = %room_id,
        "WebSocket connection requested"
    );

    // Extract JWT from Sec-WebSocket-Protocol header
    let jwt_token = headers
        .get("sec-websocket-protocol")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            warn!("Missing or invalid Sec-WebSocket-Protocol header");
            AppError::Unauthorized("Missing authentication token".to_string())
        })?;

    // Validate JWT token and get username from claims
    let claims = app_state
        .session_service
        .validate_session(jwt_token)
        .await?;
    let username = claims.username.clone();

    info!(
        room_id = %room_id,
        username = %username,
        "WebSocket authentication successful"
    );

    // Verify room exists using room service
    let room_option = app_state.room_service.get_room(&room_id).await?;
    if room_option.is_none() {
        warn!(
            room_id = %room_id,
            "Room not found, rejecting WebSocket connection"
        );
        return Err(AppError::NotFound("Room not found".to_string()));
    }

    info!(
        room_id = %room_id,
        username = %username,
        "Room verified, establishing WebSocket connection"
    );
    Ok(ws.on_upgrade(move |socket| {
        handle_websocket_connection(socket, room_id, username, claims.session_id, app_state)
    }))
}

/// Handle the upgraded WebSocket connection
async fn handle_websocket_connection(
    socket: axum::extract::ws::WebSocket,
    room_id: String,
    username: String,
    session_id: String,
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
    // Resolve stable player UUID from session id for connection identity
    let player_uuid = match app_state
        .session_service
        .get_player_uuid_by_session(&session_id)
        .await
    {
        Ok(opt) => match opt {
            Some(uuid) => uuid,
            None => {
                warn!(session_id = %session_id, "No player UUID for session");
                return;
            }
        },
        Err(_e) => {
            warn!(session_id = %session_id, "Failed to get player UUID for session");
            return;
        }
    };

    app_state
        .connection_manager
        .add_connection(player_uuid.clone(), outbound_sender.clone())
        .await;

    // Send initial room state to the newly connected player
    if let Ok(Some(room)) = app_state.room_service.get_room(&room_id).await {
        let mut mapping: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for uuid in room.get_player_uuids() {
            if let Some(name) = app_state.player_mapping.get_playername(&uuid).await {
                mapping.insert(uuid.clone(), name);
            } else {
                warn!(
                    room_id = %room_id,
                    uuid = %uuid,
                    "Player not found in mapping"
                );
            }
        }

        let initial_message = crate::websockets::messages::WebSocketMessage::players_list(
            room.get_player_uuids().clone(),
            mapping,
        );
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
        player_uuid.clone(),
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
        .remove_connection(&player_uuid)
        .await;

    // Emit disconnect event - let the event system handle the rest
    app_state
        .event_bus
        .emit_to_room(
            &room_id,
            crate::event::RoomEvent::PlayerDisconnected {
                player: player_uuid,
            },
        )
        .await;

    info!(
        room_id = %room_id,
        username = %username,
        "WebSocket disconnect event emitted"
    );
}
