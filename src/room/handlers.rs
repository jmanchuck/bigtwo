use axum::{extract::State, Json};
use std::sync::Arc;
use tracing::{info, instrument};

use super::{
    service::RoomService,
    types::{RoomCreateRequest, RoomResponse},
};
use crate::shared::{AppError, AppState};

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

    info!(
        room_id = %room.id,
        host_name = %room.host_name,
        "Room created successfully"
    );

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
}
