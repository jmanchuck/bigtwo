// Event-driven architecture components
//
// This module provides the core infrastructure for event-driven communication
// between different parts of the game server.

// Public API - what other modules can use
pub use bus::EventBus;
pub use dispatcher::EventDispatcher;
pub use events::GameEvent;
pub use handler::{EventError, EventHandler};

// Internal modules
mod bus;
mod dispatcher;
mod events;
mod handler;
