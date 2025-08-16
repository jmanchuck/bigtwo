use axum::{
    extract::{Path, State},
    Extension, Json,
};
use std::sync::Arc;
use tracing::{info, instrument};

use super::types::{JoinRoomRequest, RoomCreateRequest, RoomResponse};
use crate::{
    event::{RoomEvent, RoomSubscription},
    session::SessionClaims,
    shared::{AppError, AppState},
    websockets::WebSocketRoomSubscriber,
};

/// HTTP handler for creating a new room
///
/// POST /room
/// Returns room information with generated ID
#[instrument(name = "create_room", skip(state))]
pub async fn create_room(
    State(state): State<AppState>,
    Json(request): Json<RoomCreateRequest>,
) -> Result<Json<RoomResponse>, AppError> {
    info!(host_name = %request.host_name, "Creating new room");

    // Use injected service from app state
    let service = Arc::clone(&state.room_service);
    let room = service.create_room(request).await?;

    // Start WebSocket room subscription for this room
    let room_subscriber = Arc::new(WebSocketRoomSubscriber::new(
        Arc::clone(&state.room_service),
        Arc::clone(&state.connection_manager),
        Arc::clone(&state.game_service),
        state.event_bus.clone(),
    ));

    let room_subscription =
        RoomSubscription::new(room.id.clone(), room_subscriber, state.event_bus.clone());

    // Start the subscription background task
    let _subscription_handle = room_subscription.start().await;

    // Note: We're not storing the handle because the task will run until the room is deleted
    // and there are no more events. In a production system, you might want to store handles
    // for cleanup, but for this implementation, letting them run independently is fine.

    info!(
        room_id = %room.id,
        host_name = %room.host_name,
        "Room created successfully with WebSocket subscription active"
    );

    Ok(Json(room))
}

/// HTTP handler for listing all rooms
///
/// GET /rooms
/// Returns array of all available rooms
#[instrument(name = "list_rooms", skip(state))]
pub async fn list_rooms(
    State(state): State<AppState>,
) -> Result<Json<Vec<RoomResponse>>, AppError> {
    info!("Listing all rooms");

    // Use injected service from app state
    let service = Arc::clone(&state.room_service);
    let rooms = service.list_rooms().await?;

    info!(room_count = rooms.len(), "Rooms listed successfully");

    Ok(Json(rooms))
}

/// HTTP handler for joining a room
///
/// POST /room/{room_id}/join
///
/// Joins a player to a room and returns room information.
/// Requires valid session (X-Session-ID header)
#[instrument(name = "join_room", skip(state))]
pub async fn join_room(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Extension(claims): Extension<SessionClaims>,
    Json(_request): Json<JoinRoomRequest>,
) -> Result<Json<RoomResponse>, AppError> {
    info!(
        room_id = %room_id,
        username = %claims.username,
        session_id = %claims.session_id,
        "Player joining room"
    );

    let service = Arc::clone(&state.room_service);

    // Join the room (business logic)
    let room = service
        .join_room(room_id.clone(), claims.username.clone())
        .await?;

    // Emit room-specific event directly to room subscribers
    state
        .event_bus
        .emit_to_room(
            &room_id,
            RoomEvent::PlayerJoined {
                player: claims.username.clone(),
            },
        )
        .await;

    info!(
        room_id = %room_id,
        username = %claims.username,
        player_count = room.player_count,
        "Player joined room successfully"
    );

    // Always return JSON response with room information
    Ok(Json(room))
}

pub async fn get_room_details(
    State(state): State<AppState>,
    Path(room_id): Path<String>,
) -> Result<Json<RoomResponse>, AppError> {
    // Get room without requiring auth - just returns room info
    let service = Arc::clone(&state.room_service);
    let room = service.get_room_details(room_id.clone()).await?;

    Ok(Json(room))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room::repository::InMemoryRoomRepository;
    use crate::shared::test_utils::AppStateBuilder;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        Router,
    };
    use tower::ServiceExt; // for `oneshot`

    #[tokio::test]
    async fn test_create_room_handler() {
        // Create app state with real room repository, dummy session repository
        let room_repository = Arc::new(InMemoryRoomRepository::new());
        let app_state = AppStateBuilder::new()
            .with_room_repository(room_repository)
            .build();

        // Create router with our handler
        let app = Router::new()
            .route("/room", axum::routing::post(create_room))
            .with_state(app_state);

        // Create a request
        let request_body = r#"{"host_name": "test-player"}"#;
        let request = Request::builder()
            .method("POST")
            .uri("/room")
            .header("content-type", "application/json")
            .body(Body::from(request_body))
            .unwrap();

        // Call the handler
        let response = app.oneshot(request).await.unwrap();

        // Assert response
        assert_eq!(response.status(), StatusCode::OK);

        // Parse response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let room_response: RoomResponse = serde_json::from_slice(&body).unwrap();

        // Verify room response
        assert!(!room_response.id.is_empty());
        assert_eq!(room_response.host_name, "test-player");
        assert_eq!(room_response.status, "ONLINE");
        assert_eq!(room_response.player_count, 0); // Host doesn't auto-join
    }

    #[tokio::test]
    async fn test_create_room_handler_with_special_characters() {
        let room_repository = Arc::new(InMemoryRoomRepository::new());
        let app_state = AppStateBuilder::new()
            .with_room_repository(room_repository)
            .build();

        let app = Router::new()
            .route("/room", axum::routing::post(create_room))
            .with_state(app_state);

        let request_body = r#"{"host_name": "player-with-special-chars!@#"}"#;
        let request = Request::builder()
            .method("POST")
            .uri("/room")
            .header("content-type", "application/json")
            .body(Body::from(request_body))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let room_response: RoomResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(room_response.host_name, "player-with-special-chars!@#");
    }

    #[tokio::test]
    async fn test_create_room_handler_invalid_json() {
        let room_repository = Arc::new(InMemoryRoomRepository::new());
        let app_state = AppStateBuilder::new()
            .with_room_repository(room_repository)
            .build();

        let app = Router::new()
            .route("/room", axum::routing::post(create_room))
            .with_state(app_state);

        let request_body = r#"{"invalid": "json"}"#; // Missing host_name field
        let request = Request::builder()
            .method("POST")
            .uri("/room")
            .header("content-type", "application/json")
            .body(Body::from(request_body))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Should return 422 Unprocessable Entity for invalid JSON structure
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn test_create_room_handler_malformed_json() {
        let room_repository = Arc::new(InMemoryRoomRepository::new());
        let app_state = AppStateBuilder::new()
            .with_room_repository(room_repository)
            .build();

        let app = Router::new()
            .route("/room", axum::routing::post(create_room))
            .with_state(app_state);

        let request_body = r#"{"host_name": "test"#; // Malformed JSON
        let request = Request::builder()
            .method("POST")
            .uri("/room")
            .header("content-type", "application/json")
            .body(Body::from(request_body))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Should return 400 Bad Request for malformed JSON
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_room_handler_empty_host_name() {
        let room_repository = Arc::new(InMemoryRoomRepository::new());
        let app_state = AppStateBuilder::new()
            .with_room_repository(room_repository)
            .build();

        let app = Router::new()
            .route("/room", axum::routing::post(create_room))
            .with_state(app_state);

        let request_body = r#"{"host_name": ""}"#;
        let request = Request::builder()
            .method("POST")
            .uri("/room")
            .header("content-type", "application/json")
            .body(Body::from(request_body))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let room_response: RoomResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(room_response.host_name, "");
    }

    #[tokio::test]
    async fn test_list_rooms_handler_empty() {
        let room_repository = Arc::new(InMemoryRoomRepository::new());
        let app_state = AppStateBuilder::new()
            .with_room_repository(room_repository)
            .build();

        let app = Router::new()
            .route("/rooms", axum::routing::get(list_rooms))
            .with_state(app_state);

        let request = Request::builder()
            .method("GET")
            .uri("/rooms")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let rooms: Vec<RoomResponse> = serde_json::from_slice(&body).unwrap();

        assert!(rooms.is_empty());
    }

    #[tokio::test]
    async fn test_list_rooms_handler_with_rooms() {
        let room_repository = Arc::new(InMemoryRoomRepository::new());
        let app_state = AppStateBuilder::new()
            .with_room_repository(room_repository.clone())
            .build();

        // Create some rooms first using the service directly
        let service = Arc::clone(&app_state.room_service);
        let request1 = RoomCreateRequest {
            host_name: "host-1".to_string(),
        };
        let request2 = RoomCreateRequest {
            host_name: "host-2".to_string(),
        };
        let created_room1 = service.create_room(request1).await.unwrap();
        let created_room2 = service.create_room(request2).await.unwrap();

        let app = Router::new()
            .route("/rooms", axum::routing::get(list_rooms))
            .with_state(app_state);

        let request = Request::builder()
            .method("GET")
            .uri("/rooms")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let rooms: Vec<RoomResponse> = serde_json::from_slice(&body).unwrap();

        assert_eq!(rooms.len(), 2);

        // Verify both rooms are present (order may vary)
        let room_ids: std::collections::HashSet<String> =
            rooms.iter().map(|r| r.id.clone()).collect();
        assert!(room_ids.contains(&created_room1.id));
        assert!(room_ids.contains(&created_room2.id));

        // Verify host names are correct
        let room_hosts: std::collections::HashMap<String, String> = rooms
            .iter()
            .map(|r| (r.id.clone(), r.host_name.clone()))
            .collect();
        assert_eq!(
            room_hosts.get(&created_room1.id),
            Some(&"host-1".to_string())
        );
        assert_eq!(
            room_hosts.get(&created_room2.id),
            Some(&"host-2".to_string())
        );

        // Verify all rooms have expected structure
        for room in &rooms {
            assert!(!room.id.is_empty());
            assert_eq!(room.status, "ONLINE");
            assert_eq!(room.player_count, 0); // Host doesn't auto-join
        }
    }

    #[tokio::test]
    async fn test_list_rooms_handler_single_room() {
        let room_repository = Arc::new(InMemoryRoomRepository::new());
        let app_state = AppStateBuilder::new()
            .with_room_repository(room_repository.clone())
            .build();

        // Create one room using the service directly
        let service = Arc::clone(&app_state.room_service);
        let request = RoomCreateRequest {
            host_name: "single-host".to_string(),
        };
        let created_room = service.create_room(request).await.unwrap();

        let app = Router::new()
            .route("/rooms", axum::routing::get(list_rooms))
            .with_state(app_state);

        let request = Request::builder()
            .method("GET")
            .uri("/rooms")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let rooms: Vec<RoomResponse> = serde_json::from_slice(&body).unwrap();

        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].id, created_room.id);
        assert_eq!(rooms[0].host_name, "single-host");
        assert_eq!(rooms[0].status, "ONLINE");
        assert_eq!(rooms[0].player_count, 0); // Host doesn't auto-join
    }

    #[tokio::test]
    async fn test_join_room_handler() {
        use crate::session::SessionClaims;

        let room_repository = Arc::new(InMemoryRoomRepository::new());
        let app_state = AppStateBuilder::new()
            .with_room_repository(room_repository.clone())
            .build();

        // Create a room first using the service directly
        let service = Arc::clone(&app_state.room_service);
        let request = RoomCreateRequest {
            host_name: "test-host".to_string(),
        };
        let created_room = service.create_room(request).await.unwrap();

        // Create mock session claims
        let session_claims = SessionClaims {
            session_id: "test-session-id".to_string(),
            username: "joining-player".to_string(),
            exp: 9999999999, // Far future expiration
            iat: 1234567890, // Past issued time
        };

        // Create router with join_room handler
        let app = Router::new()
            .route("/room/:room_id/join", axum::routing::post(join_room))
            .with_state(app_state);

        // Create request with session claims in extensions (simulating middleware)
        let mut request = Request::builder()
            .method("POST")
            .uri(&format!("/room/{}/join", created_room.id))
            .header("content-type", "application/json")
            .body(Body::from("{}")) // Empty JSON body for JoinRoomRequest
            .unwrap();

        // Add session claims to request extensions (this is what the middleware would do)
        request.extensions_mut().insert(session_claims);

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Parse response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let room_response: RoomResponse = serde_json::from_slice(&body).unwrap();

        // Verify room response shows incremented player count
        assert_eq!(room_response.id, created_room.id);
        assert_eq!(room_response.host_name, "test-host");
        assert_eq!(room_response.status, "ONLINE");
        assert_eq!(room_response.player_count, 1); // Only the new player who joined
    }

    #[tokio::test]
    async fn test_join_room_handler_room_not_found() {
        let room_repository = Arc::new(InMemoryRoomRepository::new());
        let app_state = AppStateBuilder::new()
            .with_room_repository(room_repository)
            .build();

        let app = Router::new()
            .route("/room/:room_id", axum::routing::post(join_room))
            .with_state(app_state);

        let request = Request::builder()
            .method("POST")
            .uri("/room/nonexistent-room")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_join_room_handler_room_full() {
        let room_repository = Arc::new(InMemoryRoomRepository::new());
        let app_state = AppStateBuilder::new()
            .with_room_repository(room_repository.clone())
            .build();

        // Create a room and fill it to capacity using the service directly
        let service = Arc::clone(&app_state.room_service);
        let request = RoomCreateRequest {
            host_name: "test-host".to_string(),
        };
        let created_room = service.create_room(request).await.unwrap();

        // Fill room to capacity (3 more players to reach 4 total)
        for _ in 0..3 {
            service
                .join_room(created_room.id.clone(), "test-player".to_string())
                .await
                .unwrap();
        }

        let app = Router::new()
            .route("/room/:room_id", axum::routing::post(join_room))
            .with_state(app_state);

        // Try to join the full room
        let request = Request::builder()
            .method("POST")
            .uri(&format!("/room/{}", created_room.id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_join_room_handler_with_session() {
        use crate::session::SessionClaims;

        let room_repository = Arc::new(InMemoryRoomRepository::new());
        let app_state = AppStateBuilder::new()
            .with_room_repository(room_repository.clone())
            .build();

        // Create a room first using the service directly
        let service = Arc::clone(&app_state.room_service);
        let request = RoomCreateRequest {
            host_name: "test-host".to_string(),
        };
        let created_room = service.create_room(request).await.unwrap();

        // Create mock session claims
        let session_claims = SessionClaims {
            session_id: "test-session-id".to_string(),
            username: "test-player".to_string(),
            exp: 9999999999, // Far future expiration
            iat: 1234567890, // Past issued time
        };

        // Create router with join_room handler
        let app = Router::new()
            .route("/room/:room_id/join", axum::routing::post(join_room))
            .with_state(app_state);

        // Create request with session claims in extensions (simulating middleware)
        let mut request = Request::builder()
            .method("POST")
            .uri(&format!("/room/{}/join", created_room.id))
            .header("content-type", "application/json")
            .body(Body::from("{}")) // Empty JSON body for JoinRoomRequest
            .unwrap();

        // Add session claims to request extensions (this is what the middleware would do)
        request.extensions_mut().insert(session_claims);

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Parse response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let room_response: RoomResponse = serde_json::from_slice(&body).unwrap();

        // Verify room response shows incremented player count
        assert_eq!(room_response.id, created_room.id);
        assert_eq!(room_response.host_name, "test-host");
        assert_eq!(room_response.status, "ONLINE");
        assert_eq!(room_response.player_count, 1); // Only the new player (test-player) who joined
    }
}
