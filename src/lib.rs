// Library crate for Big Two game server
// This file exposes the public API for integration tests

pub mod bot;
pub mod event;
pub mod game;
pub mod room;
pub mod session;
pub mod shared;
pub mod stats;
pub mod user;
pub mod websockets;

// Re-export commonly used types for easier access in tests
pub use bot::{BotManager, BotRoomSubscriber};
pub use event::{EventBus, RoomEvent, RoomSubscription};
pub use game::GameService;
pub use room::{models::RoomModel, repository::RoomRepository};
pub use shared::AppError;
pub use stats::{
    models::*, repository::InMemoryStatsRepository, repository::StatsRepository,
    StatsRoomSubscriber, StatsService,
};
pub use user::PlayerMappingService;
pub use websockets::{ConnectionManager, WebSocketRoomSubscriber};
