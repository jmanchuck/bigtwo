use async_trait::async_trait;
use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    response::Response,
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::room::{repository::LeaveRoomResult, service::RoomService};
use crate::shared::{AppError, AppState};

use super::socket::{Connection, MessageHandler};

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

    // Create message handler (using default for now)
    let message_handler = Arc::new(DefaultMessageHandler);

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

    // Cleanup: remove from connection manager and leave the room
    app_state
        .connection_manager
        .remove_connection(&username)
        .await;

    // Remove player from room in database
    let room_service = RoomService::new(Arc::clone(&app_state.room_repository));
    match room_service
        .leave_room(room_id.clone(), username.clone())
        .await
    {
        Ok(LeaveRoomResult::Success(_)) => {
            // Emit PlayerLeft event to notify other players
            app_state
                .event_bus
                .emit_to_room(
                    &room_id,
                    crate::event::RoomEvent::PlayerLeft {
                        player: username.clone(),
                    },
                )
                .await;

            info!(
                room_id = %room_id,
                username = %username,
                "Player left room on WebSocket disconnect"
            );
        }
        Ok(LeaveRoomResult::RoomDeleted) => {
            info!(
                room_id = %room_id,
                username = %username,
                "Room deleted after last player left on WebSocket disconnect"
            );
        }
        Ok(LeaveRoomResult::PlayerNotInRoom) => {
            debug!(
                room_id = %room_id,
                username = %username,
                "Player was not in room during disconnect cleanup"
            );
        }
        Ok(LeaveRoomResult::RoomNotFound) => {
            debug!(
                room_id = %room_id,
                username = %username,
                "Room not found during disconnect cleanup"
            );
        }
        Err(e) => {
            warn!(
                room_id = %room_id,
                username = %username,
                error = ?e,
                "Error leaving room on WebSocket disconnect"
            );
        }
    }

    info!(
        room_id = %room_id,
        username = %username,
        "WebSocket cleanup completed"
    );
}
