// Public API - what other modules can use
pub use handlers::{create_room, list_rooms};

// Internal modules
mod handlers;
pub mod models;
pub mod repository;
mod service;
mod types;
