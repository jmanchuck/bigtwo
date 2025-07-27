// Event-driven architecture components
//
// Simple event system with global and room-specific events

// Public API
pub use bus::EventBus;
pub use events::RoomEvent;
pub use room_handler::{RoomEventError, RoomEventHandler};
pub use room_subscription::RoomSubscription;

// Internal modules
mod bus;
mod events;
mod room_handler;
mod room_subscription;
