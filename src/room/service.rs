use std::sync::Arc;
use tracing::{debug, info, instrument};

use super::{
    models::RoomModel,
    repository::{JoinRoomResult, RoomRepository},
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

    /// Joins an existing room by incrementing player count
    #[instrument(skip(self))]
    pub async fn join_room(
        &self,
        room_id: String,
        player_name: String,
    ) -> Result<RoomResponse, AppError> {
        debug!(room_id = %room_id, player_name = %player_name, "Attempting to join room");

        // Use the atomic try_join_room method
        let result = self
            .repository
            .try_join_room(&room_id, &player_name)
            .await?;

        match result {
            JoinRoomResult::Success(updated_room) => {
                info!(
                    room_id = %room_id,
                    player_name = %player_name,
                    new_player_count = updated_room.player_count,
                    "Player joined room successfully"
                );
                Ok(RoomResponse {
                    id: updated_room.id,
                    host_name: updated_room.host_name,
                    status: updated_room.status,
                    player_count: updated_room.player_count,
                })
            }
            JoinRoomResult::RoomNotFound => {
                Err(AppError::DatabaseError("Room not found".to_string()))
            }
            JoinRoomResult::RoomFull => Err(AppError::DatabaseError("Room is full".to_string())),
        }
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

        let result = service.list_rooms().await;
        assert!(result.is_ok());

        let rooms = result.unwrap();
        assert!(rooms.is_empty());
    }

    #[tokio::test]
    async fn test_join_room_success() {
        let repo = Arc::new(InMemoryRoomRepository::new());
        let service = RoomService::new(repo.clone());

        // First create a room
        let create_request = RoomCreateRequest {
            host_name: "host-player".to_string(),
        };
        let created_room = service.create_room(create_request).await.unwrap();

        // Now join the room
        let result = service
            .join_room(created_room.id.clone(), "player1".to_string())
            .await;
        assert!(result.is_ok());

        let joined_room = result.unwrap();
        assert_eq!(joined_room.id, created_room.id);
        assert_eq!(joined_room.host_name, "host-player");
        assert_eq!(joined_room.player_count, 2); // Host + 1 new player
        assert_eq!(joined_room.status, "ONLINE");
    }

    #[tokio::test]
    async fn test_join_room_not_found() {
        let repo = Arc::new(InMemoryRoomRepository::new());
        let service = RoomService::new(repo);

        let result = service
            .join_room("nonexistent-room".to_string(), "player1".to_string())
            .await;
        assert!(result.is_err());

        if let Err(AppError::DatabaseError(msg)) = result {
            assert_eq!(msg, "Room not found");
        } else {
            panic!("Expected DatabaseError with 'Room not found' message");
        }
    }

    #[tokio::test]
    async fn test_join_room_full() {
        let repo = Arc::new(InMemoryRoomRepository::new());
        let service = RoomService::new(repo.clone());

        // Create a room
        let create_request = RoomCreateRequest {
            host_name: "host-player".to_string(),
        };
        let created_room = service.create_room(create_request).await.unwrap();

        // Fill the room to capacity (3 more players to reach 4 total)
        for _ in 0..3 {
            service
                .join_room(created_room.id.clone(), "player1".to_string())
                .await
                .unwrap();
        }

        // Verify room is at capacity
        let room = repo.get_room(&created_room.id).await.unwrap().unwrap();
        assert_eq!(room.player_count, 4);

        // Try to join again - should fail
        let result = service
            .join_room(created_room.id.clone(), "player1".to_string())
            .await;
        assert!(result.is_err());

        if let Err(AppError::DatabaseError(msg)) = result {
            assert_eq!(msg, "Room is full");
        } else {
            panic!("Expected DatabaseError with 'Room is full' message");
        }
    }

    #[tokio::test]
    async fn test_join_room_multiple_players() {
        let repo = Arc::new(InMemoryRoomRepository::new());
        let service = RoomService::new(repo.clone());

        // Create a room
        let create_request = RoomCreateRequest {
            host_name: "host-player".to_string(),
        };
        let created_room = service.create_room(create_request).await.unwrap();
        assert_eq!(created_room.player_count, 1);

        // Add second player
        let second_join = service
            .join_room(created_room.id.clone(), "player2".to_string())
            .await
            .unwrap();
        assert_eq!(second_join.player_count, 2);

        // Add third player
        let third_join = service
            .join_room(created_room.id.clone(), "player3".to_string())
            .await
            .unwrap();
        assert_eq!(third_join.player_count, 3);

        // Add fourth player
        let fourth_join = service
            .join_room(created_room.id.clone(), "player4".to_string())
            .await
            .unwrap();
        assert_eq!(fourth_join.player_count, 4);

        // Verify final state
        let final_room = repo.get_room(&created_room.id).await.unwrap().unwrap();
        assert_eq!(final_room.player_count, 4);
        assert_eq!(final_room.host_name, "host-player");
        assert_eq!(final_room.status, "ONLINE");
    }

    #[tokio::test]
    async fn test_join_room_atomic_race_condition_protection() {
        let repo = Arc::new(InMemoryRoomRepository::new());
        let service = Arc::new(RoomService::new(repo.clone()));

        // Create a room with 3 players (1 slot remaining)
        let create_request = RoomCreateRequest {
            host_name: "host-player".to_string(),
        };
        let created_room = service.create_room(create_request).await.unwrap();

        // Add 2 more players to get to 3 total
        service
            .join_room(created_room.id.clone(), "player1".to_string())
            .await
            .unwrap();
        service
            .join_room(created_room.id.clone(), "player2".to_string())
            .await
            .unwrap();

        // Verify we're at 3 players
        let room = repo.get_room(&created_room.id).await.unwrap().unwrap();
        assert_eq!(room.player_count, 3);

        // Simulate multiple concurrent join attempts
        let service1 = Arc::clone(&service);
        let service2 = Arc::clone(&service);
        let room_id1 = created_room.id.clone();
        let room_id2 = created_room.id.clone();

        let (result1, result2) = tokio::join!(
            tokio::spawn(async move { service1.join_room(room_id1, "player1".to_string()).await }),
            tokio::spawn(async move { service2.join_room(room_id2, "player2".to_string()).await })
        );

        // Extract the actual results from the spawn handles
        let join_result1 = result1.unwrap();
        let join_result2 = result2.unwrap();

        // One should succeed, one should fail with "Room is full"
        let results = [&join_result1, &join_result2];
        let success_count = results.iter().filter(|r| r.is_ok()).count();

        let failure_count = results
            .iter()
            .filter(|r| matches!(r, Err(AppError::DatabaseError(msg)) if msg == "Room is full"))
            .count();

        assert_eq!(success_count, 1, "Exactly one join should succeed");
        assert_eq!(
            failure_count, 1,
            "Exactly one join should fail with 'Room is full'"
        );

        // Verify final room state is exactly 4 players
        let final_room = repo.get_room(&created_room.id).await.unwrap().unwrap();
        assert_eq!(
            final_room.player_count, 4,
            "Final room should have exactly 4 players"
        );
    }
}
