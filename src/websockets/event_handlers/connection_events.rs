use std::sync::Arc;
use tracing::info;

use crate::{
    event::{EventBus, RoomEvent, RoomEventError},
    room::{repository::LeaveRoomResult, service::RoomService},
    user::PlayerMappingService,
    websockets::connection_manager::ConnectionManager,
};

pub struct ConnectionEventHandlers {
    room_service: Arc<RoomService>,
    connection_manager: Arc<dyn ConnectionManager>,
    player_mapping: Arc<dyn PlayerMappingService>,
    event_bus: EventBus,
}

impl ConnectionEventHandlers {
    pub fn new(
        room_service: Arc<RoomService>,
        connection_manager: Arc<dyn ConnectionManager>,
        player_mapping: Arc<dyn PlayerMappingService>,
        event_bus: EventBus,
    ) -> Self {
        Self {
            room_service,
            connection_manager,
            player_mapping,
            event_bus,
        }
    }

    pub async fn handle_leave_request(
        &self,
        room_id: &str,
        player_uuid: &str,
    ) -> Result<(), RoomEventError> {
        info!(
            room_id = %room_id,
            player_uuid = %player_uuid,
            "Processing leave request"
        );

        let room_before = self
            .room_service
            .get_room(room_id)
            .await
            .map_err(|e| RoomEventError::HandlerError(format!("Failed to get room: {}", e)))?;

        let was_host = room_before
            .as_ref()
            .map(|room| room.host_uuid == Some(player_uuid.to_string()))
            .unwrap_or(false);

        match self
            .room_service
            .leave_room(room_id.to_string(), player_uuid.to_string())
            .await
        {
            Ok(LeaveRoomResult::Success(updated_room)) => {
                self.event_bus
                    .emit_to_room(
                        room_id,
                        RoomEvent::PlayerLeft {
                            player: player_uuid.to_string(),
                        },
                    )
                    .await;

                if was_host && updated_room.host_uuid != Some(player_uuid.to_string()) {
                    self.event_bus
                        .emit_to_room(
                            room_id,
                            RoomEvent::HostChanged {
                                old_host: player_uuid.to_string(),
                                new_host: updated_room.host_uuid.clone().unwrap(),
                            },
                        )
                        .await;
                }

                info!(
                    room_id = %room_id,
                    player_uuid = %player_uuid,
                    "Leave request processed successfully"
                );
            }
            Ok(LeaveRoomResult::RoomDeleted) => {
                info!(
                    room_id = %room_id,
                    player_uuid = %player_uuid,
                    "Room deleted after player left"
                );
            }
            Ok(_) => {
                info!(
                    room_id = %room_id,
                    player_uuid = %player_uuid,
                    "Player was not in room or room not found"
                );
            }
            Err(e) => {
                return Err(RoomEventError::HandlerError(format!(
                    "Failed to process leave: {}",
                    e
                )));
            }
        }

        Ok(())
    }
}
