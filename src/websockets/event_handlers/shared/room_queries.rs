use crate::{
    event::RoomEventError,
    room::{models::RoomModel, service::RoomService},
};
use std::sync::Arc;

pub struct RoomQueryUtils;

impl RoomQueryUtils {
    pub async fn get_room_or_error(
        room_service: &Arc<RoomService>,
        room_id: &str,
    ) -> Result<RoomModel, RoomEventError> {
        room_service
            .get_room(room_id)
            .await
            .map_err(|e| RoomEventError::HandlerError(format!("Failed to get room: {}", e)))?
            .ok_or_else(|| RoomEventError::RoomNotFound(room_id.to_string()))
    }

    pub async fn get_room_if_exists(
        room_service: &Arc<RoomService>,
        room_id: &str,
    ) -> Result<Option<RoomModel>, RoomEventError> {
        room_service
            .get_room(room_id)
            .await
            .map_err(|e| RoomEventError::HandlerError(format!("Failed to get room: {}", e)))
    }
}
