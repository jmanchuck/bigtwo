use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info};

use super::events::RoomEvent;

/// Event bus for distributing events throughout the application
#[derive(Debug, Clone)]
pub struct EventBus {
    /// Room-specific event channels: room_id -> sender
    room_channels: Arc<RwLock<HashMap<String, broadcast::Sender<RoomEvent>>>>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    /// Creates a new event bus with the specified room capacity
    pub fn new() -> Self {
        Self {
            room_channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Emits an event to all subscribers of a specific room
    pub async fn emit_to_room(&self, room_id: &str, event: RoomEvent) {
        let room_channels = self.room_channels.read().await;

        if let Some(sender) = room_channels.get(room_id) {
            match sender.send(event.clone()) {
                Ok(receiver_count) => {
                    info!(
                        room_id = %room_id,
                        receivers = receiver_count,
                        event = ?event,
                        "Room event emitted"
                    );
                }
                Err(_) => {
                    info!(room_id = %room_id, "Room event emitted with no receivers");
                }
            }
        } else {
            debug!(room_id = %room_id, "No room channel found - creating one");
            drop(room_channels);

            // Create room channel if it doesn't exist
            let mut room_channels = self.room_channels.write().await;
            let (sender, _) = broadcast::channel(100); // Room capacity
            room_channels.insert(room_id.to_string(), sender.clone());

            // Try to send again
            if sender.send(event).is_err() {
                debug!(room_id = %room_id, "Room event sent to new channel with no receivers");
            }
        }
    }

    /// Subscribe to events for a specific room
    pub async fn subscribe_to_room(&self, room_id: &str) -> broadcast::Receiver<RoomEvent> {
        let room_channels = self.room_channels.read().await;

        if let Some(sender) = room_channels.get(room_id) {
            sender.subscribe()
        } else {
            debug!(room_id = %room_id, "Creating new room channel for subscription");
            drop(room_channels);

            // Create room channel if it doesn't exist
            let mut room_channels = self.room_channels.write().await;
            let (sender, _) = broadcast::channel(100); // Room capacity
            let receiver = sender.subscribe();
            room_channels.insert(room_id.to_string(), sender);
            receiver
        }
    }

    /// Cleanup a room's event channel when the room is deleted
    /// This prevents memory leaks by removing unused channels
    pub async fn cleanup_room(&self, room_id: &str) {
        let mut room_channels = self.room_channels.write().await;
        if room_channels.remove(room_id).is_some() {
            info!(room_id = %room_id, "Room event channel cleaned up");
        }
    }
}
