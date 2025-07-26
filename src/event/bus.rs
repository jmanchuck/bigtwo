use tokio::sync::broadcast;
use tracing::{debug, warn};

use super::events::GameEvent;

/// The central event bus for the game server
///
/// This is the "town crier" that broadcasts events to all interested parties.
/// Publishers emit events here, and subscribers receive them through channels.
///
/// The EventBus uses Rust's broadcast channel internally, which provides:
/// - Multiple subscribers can receive the same event
/// - Non-blocking sends (fire-and-forget)
/// - Automatic cleanup of disconnected receivers
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<GameEvent>,
}

impl EventBus {
    /// Create a new event bus with the specified capacity
    ///
    /// The capacity determines how many events can be buffered if
    /// subscribers can't keep up with the publishing rate.
    pub fn new(capacity: usize) -> Self {
        let (sender, _receiver) = broadcast::channel(capacity);

        debug!(capacity = capacity, "Created new EventBus");

        Self { sender }
    }

    /// Create a new event bus with default capacity (1000 events)
    pub fn with_default_capacity() -> Self {
        Self::new(1000)
    }

    /// Emit an event to all subscribers
    ///
    /// This is a fire-and-forget operation - it doesn't block or fail
    /// if no one is listening. Events that can't be delivered (e.g., due
    /// to full buffers) are dropped with a warning.
    pub fn emit(&self, event: GameEvent) {
        let event_type = event.event_type();
        let room_id = event.room_id().to_string();

        match self.sender.send(event) {
            Ok(subscriber_count) => {
                debug!(
                    event_type = event_type,
                    room_id = room_id,
                    subscribers = subscriber_count,
                    "Event emitted successfully"
                );
            }
            Err(_) => {
                // This happens when there are no active subscribers
                // This is normal and not an error
                debug!(
                    event_type = event_type,
                    room_id = room_id,
                    "Event emitted but no subscribers are listening"
                );
            }
        }
    }

    /// Subscribe to events from this bus
    ///
    /// Returns a receiver that will get copies of all events published
    /// after this subscription is created. Events published before
    /// subscription are not received.
    pub fn subscribe(&self) -> broadcast::Receiver<GameEvent> {
        let receiver = self.sender.subscribe();
        debug!("New subscriber connected to EventBus");
        receiver
    }

    /// Get the number of active subscribers
    ///
    /// This is useful for monitoring and debugging
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// Check if there are any active subscribers
    pub fn has_subscribers(&self) -> bool {
        self.subscriber_count() > 0
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::with_default_capacity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_event_bus_basic_functionality() {
        let bus = EventBus::with_default_capacity();
        let mut receiver = bus.subscribe();

        // Emit an event
        let event = GameEvent::LobbyCreated {
            room_id: "test-room".to_string(),
            host: "Alice".to_string(),
        };

        bus.emit(event.clone());

        // Receive the event
        let received = timeout(Duration::from_millis(100), receiver.recv())
            .await
            .expect("Should receive event within timeout")
            .expect("Should successfully receive event");

        assert_eq!(received.room_id(), event.room_id());
        assert_eq!(received.event_type(), event.event_type());
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let bus = EventBus::with_default_capacity();
        let mut receiver1 = bus.subscribe();
        let mut receiver2 = bus.subscribe();

        assert_eq!(bus.subscriber_count(), 2);

        let event = GameEvent::PlayerJoined {
            room_id: "test-room".to_string(),
            player: "Bob".to_string(),
            current_players: vec!["Alice".to_string(), "Bob".to_string()],
        };

        bus.emit(event.clone());

        // Both receivers should get the event
        let received1 = receiver1.recv().await.unwrap();
        let received2 = receiver2.recv().await.unwrap();

        assert_eq!(received1.room_id(), event.room_id());
        assert_eq!(received2.room_id(), event.room_id());
    }

    #[tokio::test]
    async fn test_emit_with_no_subscribers() {
        let bus = EventBus::with_default_capacity();

        assert_eq!(bus.subscriber_count(), 0);
        assert!(!bus.has_subscribers());

        // This should not panic or block
        let event = GameEvent::LobbyCreated {
            room_id: "test-room".to_string(),
            host: "Alice".to_string(),
        };

        bus.emit(event);
        // Test passes if we reach this point without hanging
    }
}
