use serde_json;
use tokio::time::{sleep, Duration};

use bigtwo::{
    event::RoomEvent,
    websockets::{MessageHandler, MessageType, WebSocketMessage},
};

use super::setup::TestSetup;

// ============================================================================
// Action Helpers
// ============================================================================

impl TestSetup {
    /// Send a WebSocket message and wait for processing
    pub async fn send_message(&self, username: &str, message: WebSocketMessage) {
        let message_json = serde_json::to_string(&message).unwrap();
        self.input_handler
            .handle_message(username, "room-123", message_json)
            .await;
        sleep(Duration::from_millis(10)).await;
    }

    /// Emit a room event and wait for processing
    pub async fn emit_event(&self, event: RoomEvent) {
        self.event_bus.emit_to_room("room-123", event).await;
        sleep(Duration::from_millis(10)).await;
    }

    /// Clear all recorded messages
    pub async fn clear_messages(&self) {
        self.mock_conn_manager.clear_messages().await;
    }

    // ============================================================================
    // Convenience Action Methods
    // ============================================================================

    /// Send a chat message
    pub async fn send_chat(&self, sender: &str, content: &str) {
        self.send_message(
            sender,
            WebSocketMessage::chat(sender.to_string(), content.to_string()),
        )
        .await;
    }

    /// Send a leave message
    pub async fn send_leave(&self, player: &str) {
        self.send_message(
            player,
            WebSocketMessage::new(MessageType::Leave, serde_json::json!({})),
        )
        .await;
    }

    /// Send a start game message
    pub async fn send_start_game(&self, player: &str) {
        self.send_message(
            player,
            WebSocketMessage::new(MessageType::StartGame, serde_json::json!({})),
        )
        .await;
    }

    /// Send a move with specific cards
    pub async fn send_move(&self, player: &str, cards: Vec<&str>) {
        let move_msg =
            WebSocketMessage::new(MessageType::Move, serde_json::json!({ "cards": cards }));
        self.send_message(player, move_msg).await;
    }

    /// Send a pass move (empty cards)
    pub async fn send_pass(&self, player: &str) {
        self.send_move(player, vec![]).await;
    }
}
