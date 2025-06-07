use std::sync::Arc;
use tracing::{debug, info, instrument};

use super::{
    models::RoomModel,
    repository::RoomRepository,
    types::{RoomCreateRequest, RoomResponse},
};
use crate::shared::AppError;

/// Service for handling room business logic
pub struct RoomService {
    repository: Arc<dyn RoomRepository + Send + Sync>,
}

impl RoomService {
    pub fn new(repository: Arc<dyn RoomRepository + Send + Sync>) -> Self {
        Self { repository }
    }

    /// Creates a new room with a generated ID
    #[instrument(skip(self))]
    pub async fn create_room(&self, request: RoomCreateRequest) -> Result<RoomResponse, AppError> {
        debug!(host_name = %request.host_name, "Creating new room");

        // Create room model with generated ID
        let room_model = RoomModel::new(request.host_name);
        debug!(room_id = %room_model.id, "Generated room ID");

        // Store room in repository
        self.repository.create_room(&room_model).await?;

        info!(
            room_id = %room_model.id,
            host_name = %room_model.host_name,
            "Room created successfully"
        );

        // Convert to response
        Ok(RoomResponse {
            id: room_model.id,
            host_name: room_model.host_name,
            status: room_model.status,
            player_count: room_model.player_count,
        })
    }

    /// Lists all available rooms
    #[instrument(skip(self))]
    pub async fn list_rooms(&self) -> Result<Vec<RoomResponse>, AppError> {
        debug!("Listing all rooms");

        // Get all rooms from repository
        let rooms = self.repository.list_rooms().await?;

        info!(room_count = rooms.len(), "Rooms retrieved successfully");

        // Convert to response format
        let room_responses = rooms
            .into_iter()
            .map(|room| RoomResponse {
                id: room.id,
                host_name: room.host_name,
                status: room.status,
                player_count: room.player_count,
            })
            .collect();

        Ok(room_responses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room::repository::InMemoryRoomRepository;

    #[tokio::test]
    async fn test_create_room_success() {
        let repo = Arc::new(InMemoryRoomRepository::new());
        let service = RoomService::new(repo.clone());

        let request = RoomCreateRequest {
            host_name: "test-host".to_string(),
        };

        let result = service.create_room(request).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.host_name, "test-host");
        assert_eq!(response.status, "ONLINE");
        assert_eq!(response.player_count, 1);
        assert!(!response.id.is_empty());

        // Verify room was actually stored in repository by trying to get it
        let stored_room = repo.get_room(&response.id).await.unwrap();
        assert!(stored_room.is_some());
        assert_eq!(stored_room.unwrap().host_name, "test-host");
    }

    #[tokio::test]
    async fn test_create_room_generates_unique_ids() {
        let repo = Arc::new(InMemoryRoomRepository::new());
        let service = RoomService::new(repo.clone());

        let request1 = RoomCreateRequest {
            host_name: "host-1".to_string(),
        };
        let request2 = RoomCreateRequest {
            host_name: "host-2".to_string(),
        };

        let response1 = service.create_room(request1).await.unwrap();
        let response2 = service.create_room(request2).await.unwrap();

        // IDs should be different
        assert_ne!(response1.id, response2.id);

        // Both should be stored and retrievable
        let stored_room1 = repo.get_room(&response1.id).await.unwrap();
        assert!(stored_room1.is_some());
        assert_eq!(stored_room1.unwrap().host_name, "host-1");

        let stored_room2 = repo.get_room(&response2.id).await.unwrap();
        assert!(stored_room2.is_some());
        assert_eq!(stored_room2.unwrap().host_name, "host-2");
    }

    #[tokio::test]
    async fn test_create_room_with_empty_host_name() {
        let repo = Arc::new(InMemoryRoomRepository::new());
        let service = RoomService::new(repo);

        let request = RoomCreateRequest {
            host_name: "".to_string(),
        };

        // Should still work - validation could be added later if needed
        let result = service.create_room(request).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.host_name, "");
    }

    #[tokio::test]
    async fn test_create_room_preserves_host_name() {
        let repo = Arc::new(InMemoryRoomRepository::new());
        let service = RoomService::new(repo);

        let test_names = vec![
            "simple-name",
            "name with spaces",
            "name-with-special-chars!@#",
            "very-long-name-that-goes-on-and-on-and-on",
        ];

        for name in test_names {
            let request = RoomCreateRequest {
                host_name: name.to_string(),
            };

            let response = service.create_room(request).await.unwrap();
            assert_eq!(response.host_name, name);
        }
    }

    #[tokio::test]
    async fn test_list_rooms_empty() {
        let repo = Arc::new(InMemoryRoomRepository::new());
        let service = RoomService::new(repo);

        let rooms = service.list_rooms().await.unwrap();
        assert!(rooms.is_empty());
    }

    #[tokio::test]
    async fn test_list_rooms_single() {
        let repo = Arc::new(InMemoryRoomRepository::new());
        let service = RoomService::new(repo);

        // Create a room first
        let request = RoomCreateRequest {
            host_name: "test-host".to_string(),
        };
        let created_room = service.create_room(request).await.unwrap();

        // List rooms
        let rooms = service.list_rooms().await.unwrap();
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].id, created_room.id);
        assert_eq!(rooms[0].host_name, "test-host");
        assert_eq!(rooms[0].status, "ONLINE");
        assert_eq!(rooms[0].player_count, 1);
    }

    #[tokio::test]
    async fn test_list_rooms_multiple() {
        let repo = Arc::new(InMemoryRoomRepository::new());
        let service = RoomService::new(repo);

        // Create multiple rooms
        let request1 = RoomCreateRequest {
            host_name: "host-1".to_string(),
        };
        let request2 = RoomCreateRequest {
            host_name: "host-2".to_string(),
        };
        let request3 = RoomCreateRequest {
            host_name: "host-3".to_string(),
        };

        let room1 = service.create_room(request1).await.unwrap();
        let room2 = service.create_room(request2).await.unwrap();
        let room3 = service.create_room(request3).await.unwrap();

        // List all rooms
        let rooms = service.list_rooms().await.unwrap();
        assert_eq!(rooms.len(), 3);

        // Verify all rooms are present (order may vary)
        let room_ids: std::collections::HashSet<String> =
            rooms.iter().map(|r| r.id.clone()).collect();
        assert!(room_ids.contains(&room1.id));
        assert!(room_ids.contains(&room2.id));
        assert!(room_ids.contains(&room3.id));

        // Verify host names are correct
        let room_hosts: std::collections::HashMap<String, String> = rooms
            .iter()
            .map(|r| (r.id.clone(), r.host_name.clone()))
            .collect();
        assert_eq!(room_hosts.get(&room1.id), Some(&"host-1".to_string()));
        assert_eq!(room_hosts.get(&room2.id), Some(&"host-2".to_string()));
        assert_eq!(room_hosts.get(&room3.id), Some(&"host-3".to_string()));
    }
}
