use axum::{extract::State, Json};
use std::sync::Arc;
use tracing::{info, instrument};

use super::{service::SessionService, types::SessionResponse};
use crate::shared::{AppError, AppState};

/// HTTP handler for creating a new session
///
/// POST /session
/// Returns a JWT token as session_id and generated username
#[instrument(name = "create_session", skip(state))]
pub async fn create_session(
    State(state): State<AppState>,
) -> Result<Json<SessionResponse>, AppError> {
    info!("Creating new session");

    // Use injected repository from app state
    let service = SessionService::new(Arc::clone(&state.session_repository));
    let session = service.create_session().await?;

    info!(
        username = %session.username,
        session_id_length = session.session_id.len(),
        "Session created successfully"
    );

    Ok(Json(session))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::repository::InMemorySessionRepository;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        Router,
    };
    use tower::ServiceExt; // for `oneshot`

    #[tokio::test]
    async fn test_create_session_handler() {
        // Create app state with in-memory repository for testing
        let session_repository = Arc::new(InMemorySessionRepository::new());
        let app_state = AppState::new(session_repository);

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
        let session_response: SessionResponse = serde_json::from_slice(&body).unwrap();

        // Verify session response
        assert!(!session_response.session_id.is_empty());
        assert!(!session_response.username.is_empty());
        assert!(session_response.username.contains('-')); // Pet names have dashes
    }
}
