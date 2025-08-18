use std::sync::Arc;
use tracing::{info, warn};

use crate::{
    event::RoomEventError,
    room::service::RoomService,
    websockets::{connection_manager::ConnectionManager, messages::WebSocketMessage},
};

use super::shared::{MessageBroadcaster, RoomQueryUtils};

pub struct ChatEventHandlers {
    room_service: Arc<RoomService>,
    connection_manager: Arc<dyn ConnectionManager>,
}

impl ChatEventHandlers {
    pub fn new(
        room_service: Arc<RoomService>,
        connection_manager: Arc<dyn ConnectionManager>,
    ) -> Self {
        Self {
            room_service,
            connection_manager,
        }
    }

    pub async fn handle_chat_message(
        &self,
        room_id: &str,
        sender_uuid: &str,
        content: &str,
    ) -> Result<(), RoomEventError> {
        info!(
            room_id = %room_id,
            sender_uuid = %sender_uuid,
            "Handling chat message event"
        );

        let room = match RoomQueryUtils::get_room_if_exists(&self.room_service, room_id).await? {
            Some(room) => room,
            None => {
                warn!(room_id = %room_id, "Room was deleted, no chat notifications needed");
                return Ok(());
            }
        };

        let chat_message = WebSocketMessage::chat(sender_uuid.to_string(), content.to_string());
        MessageBroadcaster::broadcast_to_players(
            &self.connection_manager,
            room.get_player_uuids(),
            &chat_message,
        ).await?;

        Ok(())
    }
}