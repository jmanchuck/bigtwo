// Public API
pub use connection_manager::{ConnectionManager, InMemoryConnectionManager};
pub use handler::websocket_handler;
pub use websocket_room_subscriber::WebSocketRoomSubscriber;

// Internal modules
mod connection_manager;
mod handler;
mod messages;
mod socket;
mod websocket_room_subscriber;
