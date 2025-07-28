use axum::{extract::State, http::StatusCode, response::Json, Extension};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{info, instrument};

use super::{repository::SessionRepository, service::SessionService, types::SessionClaims};
use crate::shared::{AppError, AppState};

/// Creates a new user session
#[instrument(skip(state))]
pub async fn create_session(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    info!("Creating new session");

    let service = SessionService::new(Arc::clone(&state.session_repository));
    let session_response = service.create_session().await?;

    info!(
        username = %session_response.username,
        session_id_length = session_response.session_id.len(),
        "Session created successfully"
    );

    Ok(Json(json!({
        "session_id": session_response.session_id,
        "username": session_response.username
    })))
}

/// Validates the current session without side effects
/// This endpoint is specifically for session validation - no business logic
#[instrument(skip(_state))]
pub async fn validate_session(
    _state: State<AppState>,
    Extension(claims): Extension<SessionClaims>,
) -> Result<Json<Value>, AppError> {
    // If we reach here, the session is valid (middleware already validated it)
    Ok(Json(json!({
        "valid": true,
        "username": claims.username,
        "session_id": claims.session_id
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::repository::InMemorySessionRepository;
    use crate::shared::test_utils::AppStateBuilder;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        Router,
    };
    use tower::ServiceExt; // for `oneshot`

    #[tokio::test]
    async fn test_create_session_handler() {
        // Create app state with real session repository, dummy room repository
        let session_repository = Arc::new(InMemorySessionRepository::new());
        let app_state = AppStateBuilder::new()
            .with_session_repository(session_repository)
            .build();

        // Create router with our handler
        let app = Router::new()
            .route("/session", axum::routing::post(create_session))
            .with_state(app_state);

        // Create a request
        let request = Request::builder()
            .method("POST")
            .uri("/session")
            .body(Body::empty())
            .unwrap();

        // Call the handler
        let response = app.oneshot(request).await.unwrap();

        // Assert response
        assert_eq!(response.status(), StatusCode::OK);

        // Parse response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let session_response: Value = serde_json::from_slice(&body).unwrap();

        // Verify session response
        assert!(!session_response["session_id"].as_str().unwrap().is_empty());
        assert!(!session_response["username"].as_str().unwrap().is_empty());
        assert!(session_response["username"].as_str().unwrap().contains('-')); // Pet names have dashes
    }
}
