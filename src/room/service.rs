use std::sync::Arc;
use tracing::{debug, info, instrument};

use super::{
    models::RoomModel,
    repository::{JoinRoomResult, LeaveRoomResult, RoomRepository},
    types::{RoomCreateRequest, RoomResponse},
};
use crate::{shared::AppError, user::PlayerMappingService};

/// Service for handling room business logic
pub struct RoomService {
    repository: Arc<dyn RoomRepository + Send + Sync>,
    player_mapping: Arc<dyn PlayerMappingService>,
}

impl RoomService {
    pub fn new(
        repository: Arc<dyn RoomRepository + Send + Sync>,
        player_mapping: Arc<dyn PlayerMappingService>,
    ) -> Self {
        Self {
            repository,
            player_mapping,
        }
    }

    /// Creates a new room with a generated ID
    #[instrument(skip(self))]
    pub async fn create_room(&self, request: RoomCreateRequest) -> Result<RoomResponse, AppError> {
        // Create room model with generated ID
        let room_model = RoomModel::new(request.host_uuid);
        debug!(room_id = %room_model.id, "Generated room ID");

        // Store room in repository
        self.repository.create_room(&room_model).await?;

        // Map host UUID to display name
        let host_uuid = room_model.host_uuid.clone().unwrap();
        let host_name = self
            .player_mapping
            .get_playername(&host_uuid)
            .await
            .unwrap_or(host_uuid);

        // Convert to response format
        let room_response = RoomResponse {
            id: room_model.id.clone(),
            host_name,
            status: room_model.status.clone(),
            player_count: room_model.get_player_count(),
        };

        info!(
            room_id = %room_response.id,
            host_name = %room_response.host_name,
            "Room created successfully"
        );

        Ok(room_response)
    }

    /// Gets room details as a response object for API endpoints
    #[instrument(skip(self))]
    pub async fn get_room_details(&self, room_id: String) -> Result<RoomResponse, AppError> {
        let room = self
            .repository
            .get_room(&room_id)
            .await?
            .ok_or(AppError::DatabaseError("Room not found".to_string()))?;

        // Map host UUID to display name
        let host_uuid = room.host_uuid.clone().unwrap();
        let host_name = self
            .player_mapping
            .get_playername(&host_uuid)
            .await
            .unwrap_or(host_uuid);

        Ok(RoomResponse {
            id: room.id.clone(),
            host_name,
            status: room.status.clone(),
            player_count: room.get_player_count(),
        })
    }

    /// Gets the full room model for internal use (WebSocket handlers, etc.)
    #[instrument(skip(self))]
    pub async fn get_room(&self, room_id: &str) -> Result<Option<RoomModel>, AppError> {
        debug!(room_id = %room_id, "Getting room model");
        self.repository.get_room(room_id).await
    }

    /// Lists all available rooms
    #[instrument(skip(self))]
    pub async fn list_rooms(&self) -> Result<Vec<RoomResponse>, AppError> {
        debug!("Listing all rooms");

        // Get all rooms from repository
        let rooms = self.repository.list_rooms().await?;

        info!(room_count = rooms.len(), "Rooms retrieved successfully");

        // Convert to response format with UUID-to-name mapping
        let mut room_responses = Vec::new();
        for room in rooms {
            // Map host UUID to display name
            let host_uuid = room.host_uuid.clone().unwrap();
            let host_name = self
                .player_mapping
                .get_playername(&host_uuid)
                .await
                .unwrap_or(host_uuid);

            room_responses.push(RoomResponse {
                id: room.id.clone(),
                host_name,
                status: room.status.clone(),
                player_count: room.get_player_count(),
            });
        }

        Ok(room_responses)
    }

    /// Joins an existing room by incrementing player count
    #[instrument(skip(self))]
    pub async fn join_room(
        &self,
        room_id: String,
        player_uuid: String,
    ) -> Result<RoomResponse, AppError> {
        info!(room_id = %room_id, player_name = %player_uuid, "Attempting to join room");

        // Use the atomic try_join_room method
        let result = self
            .repository
            .try_join_room(&room_id, &player_uuid)
            .await?;

        match result {
            JoinRoomResult::Success(updated_room) => {
                info!(
                    room_id = %room_id,
                    player_name = %player_uuid,
                    new_player_count = updated_room.get_player_count(),
                    "Player joined room successfully"
                );
                // Map host UUID to display name
                let host_uuid = updated_room.host_uuid.clone().unwrap();
                let host_name = self
                    .player_mapping
                    .get_playername(&host_uuid)
                    .await
                    .unwrap_or(host_uuid);

                Ok(RoomResponse {
                    id: updated_room.id.clone(),
                    host_name,
                    status: updated_room.status.clone(),
                    player_count: updated_room.get_player_count(),
                })
            }
            JoinRoomResult::RoomNotFound => {
                Err(AppError::DatabaseError("Room not found".to_string()))
            }
            JoinRoomResult::RoomFull => Err(AppError::DatabaseError("Room is full".to_string())),
        }
    }

    /// Leaves a room by removing the player from the room
    #[instrument(skip(self))]
    pub async fn leave_room(
        &self,
        room_id: String,
        player_name: String,
    ) -> Result<LeaveRoomResult, AppError> {
        debug!(room_id = %room_id, player_name = %player_name, "Attempting to leave room");

        // Use the atomic leave_room method
        let result = self.repository.leave_room(&room_id, &player_name).await?;

        match &result {
            LeaveRoomResult::Success(updated_room) => {
                info!(
                    room_id = %room_id,
                    player_name = %player_name,
                    new_player_count = updated_room.get_player_count(),
                    "Player left room successfully"
                );
            }
            LeaveRoomResult::RoomDeleted => {
                info!(
                    room_id = %room_id,
                    player_name = %player_name,
                    "Room deleted after last player left"
                );
            }
            LeaveRoomResult::PlayerNotInRoom => {
                debug!(
                    room_id = %room_id,
                    player_name = %player_name,
                    "Player was not in room"
                );
            }
            LeaveRoomResult::RoomNotFound => {
                debug!(
                    room_id = %room_id,
                    player_name = %player_name,
                    "Room not found"
                );
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room::repository::InMemoryRoomRepository;

    #[tokio::test]
    async fn test_create_room_success() {
        let repo = Arc::new(InMemoryRoomRepository::new());
        let player_mapping =
            Arc::new(crate::user::mapping_service::InMemoryPlayerMappingService::new());
        // Add mapping for test UUID
        player_mapping
            .register_player(
                "550e8400-e29b-41d4-a716-446655440000".to_string(),
                "test-host".to_string(),
            )
            .await
            .unwrap();
        let service = RoomService::new(repo.clone(), player_mapping.clone());

        let request = RoomCreateRequest {
            host_uuid: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        };

        let result = service.create_room(request).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.host_name, "test-host");
        assert_eq!(response.status, "ONLINE");
        assert_eq!(response.player_count, 0); // Host doesn't auto-join
        assert!(!response.id.is_empty());

        // Verify room was actually stored in repository by trying to get it
        let stored_room = repo.get_room(&response.id).await.unwrap();
        assert!(stored_room.is_some());
        assert_eq!(
            stored_room.unwrap().host_uuid,
            Some("550e8400-e29b-41d4-a716-446655440000".to_string()) // Should store the UUID, not the name
        );
    }

    #[tokio::test]
    async fn test_create_room_generates_unique_ids() {
        let repo = Arc::new(InMemoryRoomRepository::new());
        let player_mapping =
            Arc::new(crate::user::mapping_service::InMemoryPlayerMappingService::new());
        // Add mappings for test UUIDs
        player_mapping
            .register_player(
                "550e8400-e29b-41d4-a716-446655440001".to_string(),
                "host-1".to_string(),
            )
            .await
            .unwrap();
        player_mapping
            .register_player(
                "550e8400-e29b-41d4-a716-446655440002".to_string(),
                "host-2".to_string(),
            )
            .await
            .unwrap();
        let service = RoomService::new(repo.clone(), player_mapping.clone());

        let request1 = RoomCreateRequest {
            host_uuid: "550e8400-e29b-41d4-a716-446655440001".to_string(),
        };
        let request2 = RoomCreateRequest {
            host_uuid: "550e8400-e29b-41d4-a716-446655440002".to_string(),
        };

        let response1 = service.create_room(request1).await.unwrap();
        let response2 = service.create_room(request2).await.unwrap();

        // IDs should be different
        assert_ne!(response1.id, response2.id);

        // Both should be stored and retrievable
        let stored_room1 = repo.get_room(&response1.id).await.unwrap();
        assert!(stored_room1.is_some());
        assert_eq!(
            stored_room1.unwrap().host_uuid,
            Some("550e8400-e29b-41d4-a716-446655440001".to_string())
        );

        let stored_room2 = repo.get_room(&response2.id).await.unwrap();
        assert!(stored_room2.is_some());
        assert_eq!(
            stored_room2.unwrap().host_uuid,
            Some("550e8400-e29b-41d4-a716-446655440002".to_string())
        );
    }

    #[tokio::test]
    async fn test_create_room_with_empty_host_name() {
        let repo = Arc::new(InMemoryRoomRepository::new());
        let player_mapping =
            Arc::new(crate::user::mapping_service::InMemoryPlayerMappingService::new());
        let service = RoomService::new(repo, player_mapping);

        let request = RoomCreateRequest {
            host_uuid: "".to_string(),
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
        let player_mapping =
            Arc::new(crate::user::mapping_service::InMemoryPlayerMappingService::new());
        let service = RoomService::new(repo, player_mapping.clone());

        let test_cases = vec![
            ("550e8400-e29b-41d4-a716-446655440000", "simple-name"),
            ("550e8400-e29b-41d4-a716-446655440001", "name with spaces"),
            (
                "550e8400-e29b-41d4-a716-446655440002",
                "name-with-special-chars!@#",
            ),
            (
                "550e8400-e29b-41d4-a716-446655440003",
                "very-long-name-that-goes-on-and-on-and-on",
            ),
        ];

        for (uuid, name) in test_cases {
            // Register the player mapping
            player_mapping
                .register_player(uuid.to_string(), name.to_string())
                .await
                .unwrap();

            let request = RoomCreateRequest {
                host_uuid: uuid.to_string(),
            };

            let response = service.create_room(request).await.unwrap();
            assert_eq!(response.host_name, name);
        }
    }

    #[tokio::test]
    async fn test_list_rooms() {
        let repository = Arc::new(InMemoryRoomRepository::new());
        let player_mapping =
            Arc::new(crate::user::mapping_service::InMemoryPlayerMappingService::new());
        let service = RoomService::new(repository.clone(), player_mapping.clone());

        // Create some test rooms
        let room1 = RoomModel::new("test-host-1".to_string());
        let room2 = RoomModel::new("test-host-2".to_string());

        repository.create_room(&room1).await.unwrap();
        repository.create_room(&room2).await.unwrap();

        // List all rooms
        let rooms = service.list_rooms().await.unwrap();

        assert_eq!(rooms.len(), 2);
        assert!(rooms.iter().any(|r| r.host_name == "test-host-1"));
        assert!(rooms.iter().any(|r| r.host_name == "test-host-2"));
    }

    #[tokio::test]
    async fn test_join_room_success() {
        let repository = Arc::new(InMemoryRoomRepository::new());
        let player_mapping =
            Arc::new(crate::user::mapping_service::InMemoryPlayerMappingService::new());
        // Add mapping for test UUID
        player_mapping
            .register_player(
                "550e8400-e29b-41d4-a716-446655440000".to_string(),
                "test-host".to_string(),
            )
            .await
            .unwrap();
        let service = RoomService::new(repository.clone(), player_mapping.clone());

        // Create a room first
        let create_request = RoomCreateRequest {
            host_uuid: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        };
        let created_room = service.create_room(create_request).await.unwrap();

        // Join the room
        let joined_room = service
            .join_room(created_room.id.clone(), "test-player".to_string())
            .await
            .unwrap();

        assert_eq!(joined_room.id, created_room.id);
        assert_eq!(joined_room.host_name, "test-host");
        assert_eq!(joined_room.player_count, 1); // Only the new player who joined
    }

    #[tokio::test]
    async fn test_join_nonexistent_room() {
        let repository = Arc::new(InMemoryRoomRepository::new());
        let player_mapping =
            Arc::new(crate::user::mapping_service::InMemoryPlayerMappingService::new());
        let service = RoomService::new(repository, player_mapping);

        let result = service
            .join_room("nonexistent-room".to_string(), "test-player".to_string())
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Room not found"));
    }

    #[tokio::test]
    async fn test_room_capacity_limit() {
        let repository = Arc::new(InMemoryRoomRepository::new());
        let player_mapping =
            Arc::new(crate::user::mapping_service::InMemoryPlayerMappingService::new());
        let service = RoomService::new(repository.clone(), player_mapping.clone());

        // Create a room
        let create_request = RoomCreateRequest {
            host_uuid: "test-host-uuid".to_string(),
        };
        let created_room = service.create_room(create_request).await.unwrap();

        // Add 4 players to reach capacity (4 total)
        service
            .join_room(created_room.id.clone(), "player1".to_string())
            .await
            .unwrap();
        service
            .join_room(created_room.id.clone(), "player2".to_string())
            .await
            .unwrap();
        service
            .join_room(created_room.id.clone(), "player3".to_string())
            .await
            .unwrap();
        service
            .join_room(created_room.id.clone(), "player4".to_string())
            .await
            .unwrap();

        // Verify room is at capacity
        let room = repository
            .get_room(&created_room.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(room.get_player_count(), 4); // 4 players joined (room is full)

        // Try to join again - should fail
        let result = service
            .join_room(created_room.id.clone(), "player5".to_string())
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Room is full"));
    }

    #[tokio::test]
    async fn test_multiple_room_joins_with_capacity() {
        let repository = Arc::new(InMemoryRoomRepository::new());
        let player_mapping =
            Arc::new(crate::user::mapping_service::InMemoryPlayerMappingService::new());
        // Add mapping for test UUID
        player_mapping
            .register_player(
                "550e8400-e29b-41d4-a716-446655440003".to_string(),
                "host-player".to_string(),
            )
            .await
            .unwrap();
        let service = RoomService::new(repository.clone(), player_mapping.clone());

        // Create a room
        let create_request = RoomCreateRequest {
            host_uuid: "550e8400-e29b-41d4-a716-446655440003".to_string(),
        };
        let created_room = service.create_room(create_request).await.unwrap();
        assert_eq!(created_room.player_count, 0); // Host doesn't auto-join

        // Add second player
        let second_join = service
            .join_room(created_room.id.clone(), "player2".to_string())
            .await
            .unwrap();
        assert_eq!(second_join.player_count, 1); // First player joined

        // Add third player
        let third_join = service
            .join_room(created_room.id.clone(), "player3".to_string())
            .await
            .unwrap();
        assert_eq!(third_join.player_count, 2); // Second player joined

        // Add fourth player
        let fourth_join = service
            .join_room(created_room.id.clone(), "player4".to_string())
            .await
            .unwrap();
        assert_eq!(fourth_join.player_count, 3); // Third player joined

        // Verify final state
        let final_room = repository
            .get_room(&created_room.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(final_room.get_player_count(), 3);
        assert_eq!(
            final_room.host_uuid,
            Some("550e8400-e29b-41d4-a716-446655440003".to_string())
        ); // Should store UUID
        assert_eq!(final_room.status, "ONLINE");
    }

    #[tokio::test]
    async fn test_concurrent_room_joins() {
        let repository = Arc::new(InMemoryRoomRepository::new());
        let player_mapping =
            Arc::new(crate::user::mapping_service::InMemoryPlayerMappingService::new());
        // Add mapping for test UUID
        player_mapping
            .register_player(
                "550e8400-e29b-41d4-a716-446655440003".to_string(),
                "host-player".to_string(),
            )
            .await
            .unwrap();
        let service = RoomService::new(repository.clone(), player_mapping.clone());

        // Create a room
        let create_request = RoomCreateRequest {
            host_uuid: "550e8400-e29b-41d4-a716-446655440003".to_string(),
        };
        let created_room = service.create_room(create_request).await.unwrap();

        // Manually add 2 players to get to 3 total
        service
            .join_room(created_room.id.clone(), "player2".to_string())
            .await
            .unwrap();
        service
            .join_room(created_room.id.clone(), "player3".to_string())
            .await
            .unwrap();

        // Verify we're at 2 players
        let room = repository
            .get_room(&created_room.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(room.get_player_count(), 2); // 2 players joined (host didn't auto-join)

        // Simulate multiple concurrent join attempts
        let room_id = created_room.id.clone();
        let service = Arc::new(service);

        let handles = (0..5)
            .map(|i| {
                let service = Arc::clone(&service);
                let room_id = room_id.clone();
                tokio::spawn(async move {
                    service
                        .join_room(room_id, format!("concurrent-player-{}", i))
                        .await
                })
            })
            .collect::<Vec<_>>();

        let results = futures::future::join_all(handles).await;

        // Only 2 should succeed (reaching capacity of 4), others should fail
        let successes = results.into_iter().filter_map(|r| r.unwrap().ok()).count();
        assert_eq!(successes, 2); // 2 more players can join (2 existing + 2 new = 4 total)

        // Verify final room state
        let final_room = repository
            .get_room(&created_room.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            final_room.get_player_count(),
            4,
            "Final room should have exactly 4 players"
        );
    }
}
