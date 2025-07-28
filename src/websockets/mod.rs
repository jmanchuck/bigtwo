// Public API
pub use connection_manager::{ConnectionManager, InMemoryConnectionManager};
pub use handler::{websocket_handler, DefaultMessageHandler};
pub use messages::WebSocketMessage;
pub use room_subscriber::WebSocketRoomSubscriber;
pub use socket::{Connection, MessageHandler, SocketError, SocketWrapper};

// Internal modules
mod connection_manager;
mod handler;
mod messages;
pub mod room_subscriber;
mod socket;
