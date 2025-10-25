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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room::repository::{InMemoryRoomRepository, RoomRepository};

    #[tokio::test]
    async fn test_get_room_or_error_ok_and_not_found() {
        let repo = Arc::new(InMemoryRoomRepository::new());
        let service = Arc::new(RoomService::new(repo.clone()));

        // Create a room
        let room = crate::room::models::RoomModel {
            id: "r1".to_string(),
            host_uuid: Some("h".to_string()),
            status: "ONLINE".to_string(),
            player_uuids: vec![],
            ready_players: vec![],
        };
        repo.create_room(&room).await.unwrap();

        // ok
        let got = RoomQueryUtils::get_room_or_error(&service, "r1")
            .await
            .unwrap();
        assert_eq!(got.id, "r1");

        // not found
        let err = RoomQueryUtils::get_room_or_error(&service, "missing")
            .await
            .unwrap_err();
        match err {
            RoomEventError::RoomNotFound(id) => assert_eq!(id, "missing".to_string()),
            _ => panic!("unexpected"),
        }
    }

    #[tokio::test]
    async fn test_get_room_if_exists() {
        let repo = Arc::new(InMemoryRoomRepository::new());
        let service = Arc::new(RoomService::new(repo.clone()));

        // None initially
        let none = RoomQueryUtils::get_room_if_exists(&service, "x")
            .await
            .unwrap();
        assert!(none.is_none());

        // Create one and fetch
        let room = crate::room::models::RoomModel {
            id: "r2".to_string(),
            host_uuid: Some("h".to_string()),
            status: "ONLINE".to_string(),
            player_uuids: vec![],
            ready_players: vec![],
        };
        repo.create_room(&room).await.unwrap();
        let some = RoomQueryUtils::get_room_if_exists(&service, "r2")
            .await
            .unwrap();
        assert!(some.is_some());
        assert_eq!(some.unwrap().id, "r2");
    }
}
