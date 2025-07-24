use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    response::Response,
};
use serde::Deserialize;
use tracing::{info, instrument, warn};

use crate::shared::{AppError, AppState};

use crate::game::Game;

/// Query parameters for WebSocket connection
#[derive(Debug, Deserialize)]
pub struct WebSocketQuery {
    pub player_name: String,
    pub session_id: Option<String>,
}

/// WebSocket upgrade handler
///
/// GET /ws/{room_id}?player_name=X&session_id=Y
/// Upgrades HTTP connection to WebSocket for real-time room communication
#[instrument(name = "websocket_handler", skip(state, ws))]
pub async fn websocket_handler(
    Path(room_id): Path<String>,
    Query(query): Query<WebSocketQuery>,
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> Result<Response, AppError> {
    info!(
        room_id = %room_id,
        player_name = %query.player_name,
        session_id = ?query.session_id,
        "WebSocket connection attempt"
    );

    // TODO: Validate session exists
    // TODO: Validate room exists
    // TODO: Check if player can join room

    info!(
        room_id = %room_id,
        player_name = %query.player_name,
        "WebSocket connection established"
    );

    Ok(ws.on_upgrade(move |socket| handle_websocket(socket, room_id, query.player_name)))
}

async fn handle_join_room(room_id: String, player_name: String) -> Result<(), AppError> {
    Ok(())
}

/// Handle individual WebSocket connection
async fn handle_websocket(
    socket: axum::extract::ws::WebSocket,
    room_id: String,
    player_name: String,
) {
    info!(
        room_id = %room_id,
        player_name = %player_name,
        "WebSocket connection handler started"
    );

    // TODO: Add to connection manager
    // TODO: Send PLAYERS_LIST message
    // TODO: Broadcast join notification
    // TODO: Handle incoming messages
    // TODO: Clean up on disconnect

    // For now, just log that we're handling the connection
    warn!("WebSocket handler not fully implemented yet");
}
