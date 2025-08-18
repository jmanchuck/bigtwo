use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::{
    event::RoomEventError,
    room::service::RoomService,
    user::PlayerMappingService,
    websockets::{connection_manager::ConnectionManager, messages::WebSocketMessage},
};

use super::shared::{MessageBroadcaster, PlayerMappingUtils, RoomQueryUtils};

pub struct RoomEventHandlers {
    room_service: Arc<RoomService>,
    connection_manager: Arc<dyn ConnectionManager>,
    player_mapping: Arc<dyn PlayerMappingService>,
}

impl RoomEventHandlers {
    pub fn new(
        room_service: Arc<RoomService>,
        connection_manager: Arc<dyn ConnectionManager>,
        player_mapping: Arc<dyn PlayerMappingService>,
    ) -> Self {
        Self {
            room_service,
            connection_manager,
            player_mapping,
        }
    }

    pub async fn handle_player_joined(&self, room_id: &str) -> Result<(), RoomEventError> {
        debug!(room_id = %room_id, "Handling player joined event");

        let room = RoomQueryUtils::get_room_or_error(&self.room_service, room_id).await?;

        let mapping = PlayerMappingUtils::build_uuid_to_name_mapping(
            &self.player_mapping,
            room.get_player_uuids(),
        )
        .await;

        let ws_message = WebSocketMessage::players_list(room.get_player_uuids().clone(), mapping);

        MessageBroadcaster::broadcast_to_players(
            &self.connection_manager,
            room.get_player_uuids(),
            &ws_message,
        )
        .await?;

        debug!(
            room_id = %room_id,
            players_notified = room.get_player_uuids().len(),
            "Player joined notification sent to all room players"
        );

        Ok(())
    }

    pub async fn handle_player_left(
        &self,
        room_id: &str,
        uuid: &str,
    ) -> Result<(), RoomEventError> {
        debug!(
            room_id = %room_id,
            uuid = %uuid,
            "Handling player left event"
        );

        let room = match RoomQueryUtils::get_room_if_exists(&self.room_service, room_id).await? {
            Some(room) => room,
            None => {
                debug!(room_id = %room_id, "Room was deleted, no notifications needed");
                return Ok(());
            }
        };

        let player_name = PlayerMappingUtils::get_player_name(&self.player_mapping, uuid)
            .await
            .unwrap_or_else(|| uuid.to_string());

        let leave_message = WebSocketMessage::leave(player_name);
        MessageBroadcaster::broadcast_to_players(
            &self.connection_manager,
            room.get_player_uuids(),
            &leave_message,
        )
        .await?;

        let mapping = PlayerMappingUtils::build_uuid_to_name_mapping(
            &self.player_mapping,
            room.get_player_uuids(),
        )
        .await;

        let players_list_message =
            WebSocketMessage::players_list(room.get_player_uuids().clone(), mapping);
        MessageBroadcaster::broadcast_to_players(
            &self.connection_manager,
            room.get_player_uuids(),
            &players_list_message,
        )
        .await?;

        Ok(())
    }

    pub async fn handle_host_changed(
        &self,
        room_id: &str,
        old_host_uuid: &str,
        new_host_uuid: &str,
    ) -> Result<(), RoomEventError> {
        info!(
            room_id = %room_id,
            old_host_uuid = %old_host_uuid,
            new_host_uuid = %new_host_uuid,
            "Handling host changed event"
        );

        let room = match RoomQueryUtils::get_room_if_exists(&self.room_service, room_id).await? {
            Some(room) => room,
            None => {
                warn!(room_id = %room_id, "Room was deleted, no host change notifications needed");
                return Ok(());
            }
        };

        let new_host_name =
            PlayerMappingUtils::get_player_name(&self.player_mapping, new_host_uuid)
                .await
                .unwrap_or_else(|| new_host_uuid.to_string());

        let host_change_message = WebSocketMessage::host_change(new_host_name);
        MessageBroadcaster::broadcast_to_players(
            &self.connection_manager,
            room.get_player_uuids(),
            &host_change_message,
        )
        .await?;

        info!(
            room_id = %room_id,
            old_host_uuid = %old_host_uuid,
            new_host_uuid = %new_host_uuid,
            players_notified = room.get_player_uuids().len(),
            "Host change notification sent to all room players"
        );

        Ok(())
    }
}
