use async_trait::async_trait;
use axum::{
    extract::{Extension, Path, State, WebSocketUpgrade},
    response::Response,
};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::{session::SessionClaims, shared::AppState};

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

/// Minimal WebSocket upgrade endpoint
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Path(room_id): Path<String>,
    State(app_state): State<AppState>,
    Extension(claims): Extension<SessionClaims>,
) -> Response {
    let username = claims.username.clone();

    info!(
        room_id = %room_id,
        username = %username,
        "WebSocket upgrade requested"
    );

    ws.on_upgrade(move |socket| handle_websocket_connection(socket, room_id, username, app_state))
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
        .add_connection(username.clone(), outbound_sender)
        .await;

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

    // Cleanup: remove from connection manager
    app_state
        .connection_manager
        .remove_connection(&username)
        .await;

    info!(
        room_id = %room_id,
        username = %username,
        "WebSocket cleanup completed"
    );
}
