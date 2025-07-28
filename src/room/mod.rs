// Public API - what other modules can use
pub use handlers::{create_room, join_room, list_rooms};

// Internal modules
mod handlers;
pub mod models;
pub mod repository;
pub mod service;
mod types;
