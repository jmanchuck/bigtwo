use serde::{Deserialize, Serialize};

/// Request payload for creating a new room
#[derive(Debug, Deserialize)]
pub struct RoomCreateRequest {
    pub host_name: String,
}

/// Response for room creation and room information
#[derive(Debug, Serialize, Deserialize)]
pub struct RoomResponse {
    pub id: String,
    pub host_name: String,
    pub status: String,
    pub player_count: i32,
}
