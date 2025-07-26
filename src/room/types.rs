use serde::{Deserialize, Serialize};

/// Request payload for creating a new room
#[derive(Debug, Deserialize)]
pub struct RoomCreateRequest {
    pub host_name: String,
}

/// Request payload for joining a room
/// Currently empty since player info comes from session,
/// but allows for future expansion (e.g., join as spectator)
#[derive(Debug, Deserialize)]
pub struct JoinRoomRequest {
    // Future fields might include:
    // pub join_as_spectator: Option<bool>,
}

/// Response for room creation and room information
#[derive(Debug, Serialize, Deserialize)]
pub struct RoomResponse {
    pub id: String,
    pub host_name: String,
    pub status: String,
    pub player_count: i32,
}
