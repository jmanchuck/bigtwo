use async_trait::async_trait;
use thiserror::Error;

use super::events::RoomEvent;

/// Errors that can occur when handling room events
#[derive(Debug, Error)]
pub enum RoomEventError {
    #[error("Room not found: {0}")]
    RoomNotFound(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Handler error: {0}")]
    HandlerError(String),
}

/// Trait for components that can handle room events
///
/// This provides a clean interface for reacting to room-specific events
/// without being tied to WebSocket or connection specifics.
#[async_trait]
pub trait RoomEventHandler: Send + Sync {
    /// Handle a room event
    ///
    /// The handler should:
    /// - Process the event appropriately for its purpose
    /// - Handle any necessary state updates or notifications
    /// - Return Ok(()) on success or RoomEventError on failure
    async fn handle_room_event(
        &self,
        room_id: &str,
        event: RoomEvent,
    ) -> Result<(), RoomEventError>;

    /// Get a human-readable name for this handler (for logging/debugging)
    fn handler_name(&self) -> &'static str;
}
