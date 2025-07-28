use axum::{
    extract::{Path, State, WebSocketUpgrade},
    response::{IntoResponse, Response},
    Extension, Json,
};
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

use super::{
    service::RoomService,
    types::{JoinRoomRequest, RoomCreateRequest, RoomResponse},
};
use crate::{
    event::{RoomEvent, RoomSubscription},
    session::SessionClaims,
    shared::{AppError, AppState},
    websockets::{room_subscriber::WebSocketRoomSubscriber, Connection, DefaultMessageHandler},
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

    // Use injected repository from app state
    let service = RoomService::new(Arc::clone(&state.room_repository));
    let room = service.create_room(request).await?;

    // Start WebSocket room subscription for this room
    let room_subscriber = Arc::new(WebSocketRoomSubscriber::new(
        Arc::clone(&state.room_repository),
        Arc::clone(&state.connection_manager),
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

    // Use injected repository from app state
    let service = RoomService::new(Arc::clone(&state.room_repository));
    let rooms = service.list_rooms().await?;

    info!(room_count = rooms.len(), "Rooms listed successfully");

    Ok(Json(rooms))
}

/// HTTP handler for joining a room with optional WebSocket upgrade
///
/// POST /room/{room_id}/join
///
/// Supports two modes:
/// 1. HTTP: Returns JSON response with room info
/// 2. WebSocket: Upgrades connection + returns room info via WebSocket
///
/// Requires valid session (X-Session-ID header)
#[instrument(name = "join_room", skip(state))]
pub async fn join_room(
    ws: Option<WebSocketUpgrade>,
    State(state): State<AppState>,
    Path(room_id): Path<String>,
    Extension(claims): Extension<SessionClaims>,
    Json(_request): Json<JoinRoomRequest>,
) -> Result<Response, AppError> {
    info!(
        room_id = %room_id,
        username = %claims.username,
        session_id = %claims.session_id,
        websocket_requested = ws.is_some(),
        "Player joining room"
    );

    let service = RoomService::new(Arc::clone(&state.room_repository));

    // 1. Join the room (business logic)
    let room = service
        .join_room(room_id.clone(), claims.username.clone())
        .await?;

    // 2. Emit room-specific event directly to room subscribers
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

    // 3. Return response based on client request type
    match ws {
        Some(websocket_upgrade) => {
            // Client wants WebSocket connection
            info!(
                room_id = %room_id,
                username = %claims.username,
                "Upgrading to WebSocket connection"
            );

            let response = websocket_upgrade.on_upgrade(move |socket| {
                handle_websocket_connection(socket, room_id, claims.username, state)
            });

            Ok(response)
        }
        None => {
            // Client wants HTTP JSON response
            Ok(Json(room).into_response())
        }
    }
}

/// Handle the upgraded WebSocket connection after room join
async fn handle_websocket_connection(
    socket: axum::extract::ws::WebSocket,
    room_id: String,
    username: String,
    app_state: AppState,
) {
    use tokio::sync::mpsc;
    use tracing::{info, warn};

    info!(
        room_id = %room_id,
        username = %username,
        "WebSocket connection established after room join"
    );

    // Create the outbound channel (app -> client)
    let (outbound_sender, outbound_receiver) = mpsc::unbounded_channel::<String>();

    // Register connection with the connection manager
    app_state
        .connection_manager
        .add_connection(username.clone(), outbound_sender)
        .await;

    // Create message handler (using default for now)
    let message_handler = Arc::new(DefaultMessageHandler);

    // Create and run the connection
    let connection = Connection::new(
        username.clone(),
        room_id.clone(),
        Box::new(socket),
        outbound_receiver,
        message_handler,
    );

    // Run the connection until disconnect
    match connection.run().await {
        Ok(()) => {
            info!(
                room_id = %room_id,
                username = %username,
                "WebSocket connection closed cleanly"
            );
        }
        Err(e) => {
            warn!(
                room_id = %room_id,
                username = %username,
                error = ?e,
                "WebSocket connection error"
            );
        }
    }

    // Cleanup: remove from connection manager and leave the room
    app_state
        .connection_manager
        .remove_connection(&username)
        .await;

    // Remove player from room in database
    use crate::room::{repository::LeaveRoomResult, service::RoomService};
    let room_service = RoomService::new(Arc::clone(&app_state.room_repository));

    match room_service
        .leave_room(room_id.clone(), username.clone())
        .await
    {
        Ok(LeaveRoomResult::Success(_)) => {
            // Emit PlayerLeft event to notify other players
            app_state
                .event_bus
                .emit_to_room(
                    &room_id,
                    crate::event::RoomEvent::PlayerLeft {
                        player: username.clone(),
                    },
                )
                .await;

            info!(
                room_id = %room_id,
                username = %username,
                "Player left room via WebSocket disconnect"
            );
        }
        Ok(LeaveRoomResult::RoomDeleted) => {
            info!(
                room_id = %room_id,
                username = %username,
                "Room deleted - last player disconnected"
            );
        }
        Ok(LeaveRoomResult::PlayerNotInRoom) => {
            debug!(
                room_id = %room_id,
                username = %username,
                "Player was not in room during disconnect cleanup"
            );
        }
        Ok(LeaveRoomResult::RoomNotFound) => {
            debug!(
                room_id = %room_id,
                username = %username,
                "Room not found during disconnect cleanup"
            );
        }
        Err(e) => {
            warn!(
                room_id = %room_id,
                username = %username,
                error = ?e,
                "Failed to remove player from room during disconnect"
            );
        }
    }

    info!(
        room_id = %room_id,
        username = %username,
        "WebSocket cleanup completed"
    );
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
        assert_eq!(room_response.player_count, 1);
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
        let service = RoomService::new(room_repository);
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
            assert_eq!(room.player_count, 1);
        }
    }

    #[tokio::test]
    async fn test_list_rooms_handler_single_room() {
        let room_repository = Arc::new(InMemoryRoomRepository::new());
        let app_state = AppStateBuilder::new()
            .with_room_repository(room_repository.clone())
            .build();

        // Create one room using the service directly
        let service = RoomService::new(room_repository);
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
        assert_eq!(rooms[0].player_count, 1);
    }

    #[tokio::test]
    async fn test_join_room_handler() {
        use crate::session::SessionClaims;

        let room_repository = Arc::new(InMemoryRoomRepository::new());
        let app_state = AppStateBuilder::new()
            .with_room_repository(room_repository.clone())
            .build();

        // Create a room first using the service directly
        let service = RoomService::new(room_repository);
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
        assert_eq!(room_response.player_count, 2); // Host + 1 new player
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
        let service = RoomService::new(room_repository);
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
        let service = RoomService::new(room_repository);
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
        assert_eq!(room_response.player_count, 2); // Host + 1 new player (test-player)
    }
}
