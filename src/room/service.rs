use std::sync::Arc;
use tracing::{debug, info, instrument};

use super::{
    models::RoomModel,
    repository::{JoinRoomResult, LeaveRoomResult, RoomRepository},
    types::RoomCreateRequest,
};
use crate::{
    event::{EventBus, RoomSubscription},
    game::GameService,
    shared::AppError,
    user::PlayerMappingService,
    websockets::{ConnectionManager, WebSocketRoomSubscriber},
};

/// Service for handling room business logic
pub struct RoomService {
    repository: Arc<dyn RoomRepository + Send + Sync>,
    // Dependencies for WebSocket subscription setup
    connection_manager: Option<Arc<dyn ConnectionManager>>,
    game_service: Option<Arc<GameService>>,
    player_mapping: Option<Arc<dyn PlayerMappingService>>,
    event_bus: Option<EventBus>,
}

impl RoomService {
    pub fn new(repository: Arc<dyn RoomRepository + Send + Sync>) -> Self {
        Self {
            repository,
            connection_manager: None,
            game_service: None,
            player_mapping: None,
            event_bus: None,
        }
    }

    /// Constructor with all dependencies for WebSocket subscription support
    pub fn new_with_subscription_deps(
        repository: Arc<dyn RoomRepository + Send + Sync>,
        connection_manager: Arc<dyn ConnectionManager>,
        game_service: Arc<GameService>,
        player_mapping: Arc<dyn PlayerMappingService>,
        event_bus: EventBus,
    ) -> Self {
        Self {
            repository,
            connection_manager: Some(connection_manager),
            game_service: Some(game_service),
            player_mapping: Some(player_mapping),
            event_bus: Some(event_bus),
        }
    }

    /// Creates a new room with a generated ID
    #[instrument(skip(self))]
    pub async fn create_room(&self, request: RoomCreateRequest) -> Result<RoomModel, AppError> {
        // Create room model with generated ID
        let room_model = RoomModel::new(request.host_uuid);
        debug!(room_id = %room_model.id, "Generated room ID");

        // Store room in repository
        self.repository.create_room(&room_model).await?;

        info!(room_id = %room_model.id, "Room created successfully");

        Ok(room_model)
    }

    /// Creates a new room and sets up WebSocket subscription
    #[instrument(skip(self))]
    pub async fn create_room_with_subscription(
        &self,
        request: RoomCreateRequest,
    ) -> Result<RoomModel, AppError> {
        // Create room using existing method
        let room_model = self.create_room(request).await?;

        // Set up WebSocket subscription if dependencies are available
        self.setup_room_subscription(&room_model.id).await?;

        Ok(room_model)
    }

    /// Sets up WebSocket subscription for a room
    #[instrument(skip(self))]
    async fn setup_room_subscription(&self, room_id: &str) -> Result<(), AppError> {
        // Check if all dependencies are available
        let connection_manager = self
            .connection_manager
            .as_ref()
            .ok_or_else(|| AppError::Internal)?;
        let game_service = self
            .game_service
            .as_ref()
            .ok_or_else(|| AppError::Internal)?;
        let player_mapping = self
            .player_mapping
            .as_ref()
            .ok_or_else(|| AppError::Internal)?;
        let event_bus = self.event_bus.as_ref().ok_or_else(|| AppError::Internal)?;

        info!(room_id = %room_id, "Setting up WebSocket subscription");

        // Create a simple RoomService for the subscriber (without subscription dependencies to avoid cycles)
        let subscriber_room_service = Arc::new(Self::new(Arc::clone(&self.repository)));

        // Create WebSocket room subscriber
        let room_subscriber = Arc::new(WebSocketRoomSubscriber::new(
            subscriber_room_service,
            Arc::clone(connection_manager),
            Arc::clone(game_service),
            Arc::clone(player_mapping),
            event_bus.clone(),
        ));

        // Create and start room subscription
        let room_subscription =
            RoomSubscription::new(room_id.to_string(), room_subscriber, event_bus.clone());

        // Start the subscription background task
        let _subscription_handle = room_subscription.start().await;

        info!(room_id = %room_id, "WebSocket subscription active");

        Ok(())
    }

    /// Gets room details as a response object for API endpoints
    #[instrument(skip(self))]
    pub async fn get_room_details(&self, room_id: String) -> Result<RoomModel, AppError> {
        let room = self
            .repository
            .get_room(&room_id)
            .await?
            .ok_or(AppError::DatabaseError("Room not found".to_string()))?;

        Ok(room)
    }

    /// Gets the full room model for internal use (WebSocket handlers, etc.)
    #[instrument(skip(self))]
    pub async fn get_room(&self, room_id: &str) -> Result<Option<RoomModel>, AppError> {
        debug!(room_id = %room_id, "Getting room model");
        self.repository.get_room(room_id).await
    }

    /// Lists all available rooms
    #[instrument(skip(self))]
    pub async fn list_rooms(&self) -> Result<Vec<RoomModel>, AppError> {
        debug!("Listing all rooms");

        // Get all rooms from repository
        let rooms = self.repository.list_rooms().await?;

        info!(room_count = rooms.len(), "Rooms retrieved successfully");

        Ok(rooms)
    }

    /// Joins an existing room by incrementing player count
    #[instrument(skip(self))]
    pub async fn join_room(
        &self,
        room_id: String,
        player_uuid: String,
    ) -> Result<RoomModel, AppError> {
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
                Ok(updated_room)
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
        player_uuid: String,
    ) -> Result<LeaveRoomResult, AppError> {
        debug!(room_id = %room_id, player_uuid = %player_uuid, "Attempting to leave room");

        // Use the atomic leave_room method
        let result = self.repository.leave_room(&room_id, &player_uuid).await?;

        match &result {
            LeaveRoomResult::Success(updated_room) => {
                info!(
                    room_id = %room_id,
                    player_uuid = %player_uuid,
                    new_player_count = updated_room.get_player_count(),
                    "Player left room successfully"
                );
            }
            LeaveRoomResult::RoomDeleted => {
                info!(
                    room_id = %room_id,
                    player_uuid = %player_uuid,
                    "Room deleted after last player left"
                );
            }
            LeaveRoomResult::PlayerNotInRoom => {
                debug!(
                    room_id = %room_id,
                    player_uuid = %player_uuid,
                    "Player was not in room"
                );
            }
            LeaveRoomResult::RoomNotFound => {
                debug!(
                    room_id = %room_id,
                    player_uuid = %player_uuid,
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
        let service = RoomService::new(repo.clone());

        let request = RoomCreateRequest {
            host_uuid: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        };

        let result = service.create_room(request).await;
        assert!(result.is_ok());

        let created = result.unwrap();
        assert_eq!(created.status, "ONLINE");
        assert_eq!(created.get_player_count(), 0); // Host doesn't auto-join
        assert!(!created.id.is_empty());

        // Verify room was actually stored in repository by trying to get it
        let stored_room = repo.get_room(&created.id).await.unwrap();
        assert!(stored_room.is_some());
        assert_eq!(
            stored_room.unwrap().host_uuid,
            Some("550e8400-e29b-41d4-a716-446655440000".to_string()) // Should store the UUID, not the name
        );
    }

    #[tokio::test]
    async fn test_create_room_generates_unique_ids() {
        let repo = Arc::new(InMemoryRoomRepository::new());
        let service = RoomService::new(repo.clone());

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
        let service = RoomService::new(repo);

        let request = RoomCreateRequest {
            host_uuid: "".to_string(),
        };

        // Should still work - validation could be added later if needed
        let result = service.create_room(request).await;
        assert!(result.is_ok());

        let created = result.unwrap();
        assert_eq!(created.host_uuid, Some("".to_string()));
    }

    #[tokio::test]
    async fn test_create_room_preserves_host_name() {
        let repo = Arc::new(InMemoryRoomRepository::new());
        let service = RoomService::new(repo);

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

        for (uuid, _name) in test_cases {
            let request = RoomCreateRequest {
                host_uuid: uuid.to_string(),
            };

            let created = service.create_room(request).await.unwrap();
            assert_eq!(created.host_uuid, Some(uuid.to_string()));
        }
    }

    #[tokio::test]
    async fn test_list_rooms() {
        let repository = Arc::new(InMemoryRoomRepository::new());
        let service = RoomService::new(repository.clone());

        // Create some test rooms
        let room1 = RoomModel::new("test-host-1".to_string());
        let room2 = RoomModel::new("test-host-2".to_string());

        repository.create_room(&room1).await.unwrap();
        repository.create_room(&room2).await.unwrap();

        // List all rooms
        let rooms = service.list_rooms().await.unwrap();

        assert_eq!(rooms.len(), 2);
        assert!(rooms
            .iter()
            .any(|r| r.host_uuid == Some("test-host-1".to_string())));
        assert!(rooms
            .iter()
            .any(|r| r.host_uuid == Some("test-host-2".to_string())));
    }

    #[tokio::test]
    async fn test_join_room_success() {
        let repository = Arc::new(InMemoryRoomRepository::new());
        let service = RoomService::new(repository.clone());

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
        assert_eq!(joined_room.get_player_count(), 1); // Only the new player who joined
    }

    #[tokio::test]
    async fn test_join_nonexistent_room() {
        let repository = Arc::new(InMemoryRoomRepository::new());
        let service = RoomService::new(repository);

        let result = service
            .join_room("nonexistent-room".to_string(), "test-player".to_string())
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Room not found"));
    }

    #[tokio::test]
    async fn test_room_capacity_limit() {
        let repository = Arc::new(InMemoryRoomRepository::new());
        let service = RoomService::new(repository.clone());

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
        let service = RoomService::new(repository.clone());

        // Create a room
        let create_request = RoomCreateRequest {
            host_uuid: "550e8400-e29b-41d4-a716-446655440003".to_string(),
        };
        let created_room = service.create_room(create_request).await.unwrap();
        assert_eq!(created_room.get_player_count(), 0); // Host doesn't auto-join

        // Add second player
        let second_join = service
            .join_room(created_room.id.clone(), "player2".to_string())
            .await
            .unwrap();
        assert_eq!(second_join.get_player_count(), 1); // First player joined

        // Add third player
        let third_join = service
            .join_room(created_room.id.clone(), "player3".to_string())
            .await
            .unwrap();
        assert_eq!(third_join.get_player_count(), 2); // Second player joined

        // Add fourth player
        let fourth_join = service
            .join_room(created_room.id.clone(), "player4".to_string())
            .await
            .unwrap();
        assert_eq!(fourth_join.get_player_count(), 3); // Third player joined

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
        let service = RoomService::new(repository.clone());

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
