use crate::{
    event::RoomEventError,
    websockets::{connection_manager::ConnectionManager, messages::WebSocketMessage},
};
use std::sync::Arc;

pub struct MessageBroadcaster;

impl MessageBroadcaster {
    pub async fn broadcast_to_players(
        connection_manager: &Arc<dyn ConnectionManager>,
        player_uuids: &[String],
        message: &WebSocketMessage,
    ) -> Result<(), RoomEventError> {
        let message_json = serde_json::to_string(message).map_err(|e| {
            RoomEventError::HandlerError(format!("Failed to serialize message: {}", e))
        })?;

        for uuid in player_uuids {
            connection_manager.send_to_player(uuid, &message_json).await;
        }

        Ok(())
    }

    pub async fn broadcast_to_room_via_uuids(
        connection_manager: &Arc<dyn ConnectionManager>,
        player_uuids: &[String],
        message_json: &str,
    ) {
        for uuid in player_uuids {
            connection_manager.send_to_player(uuid, message_json).await;
        }
    }
}
