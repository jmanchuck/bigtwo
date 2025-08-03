// Library crate for Big Two game server
// This file exposes the public API for integration tests

pub mod event;
pub mod game;
pub mod room;
pub mod session;
pub mod shared;
pub mod websockets;

// Re-export commonly used types for easier access in tests
pub use event::{EventBus, RoomEvent, RoomSubscription};
pub use game::GameManager;
pub use room::{models::RoomModel, repository::RoomRepository};
pub use shared::AppError;
pub use websockets::{
    ConnectionManager, MessageHandler, MessageType, WebSocketMessage, WebSocketRoomSubscriber,
    WebsocketReceiveHandler,
};
