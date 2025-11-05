use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use tracing::{info, instrument, warn};

use crate::shared::{AppError, AppState};

/// JWT authentication middleware - validates Authorization Bearer header and adds SessionClaims to request.
/// Usage: .layer(middleware::from_fn_with_state(app_state.clone(), session::jwt_auth))
/// Handlers can then extract Extension(claims): Extension<SessionClaims>.
#[instrument(skip(state, req, next))]
pub async fn jwt_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    info!(
        "JWT authentication middleware triggered for request {}",
        req.uri()
    );

    // Use session service from app state
    let service = &state.session_service;

    // Extract token from Authorization Bearer header
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .ok_or_else(|| {
            warn!("Missing Authorization header in request");
            AppError::Unauthorized("Missing authorization header".to_string())
        })?;

    // Extract Bearer token
    let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
        warn!("Invalid Authorization header format (expected Bearer token)");
        AppError::Unauthorized("Invalid authorization header format".to_string())
    })?;

    info!("Extracted token from Authorization Bearer header");

    // Validate token, log error if it fails
    let claims = match service.validate_session(token).await {
        Ok(claims) => claims,
        Err(e) => {
            warn!("JWT authentication failed: {}", e);
            return Err(e);
        }
    };

    info!(
        username = %claims.username,
        session_id = %claims.session_id,
        "Authentication successful, adding claims to request"
    );

    // Add claims to request extensions for handlers to use
    req.extensions_mut().insert(claims);

    // Continue to next middleware/handler
    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::repository::InMemorySessionRepository;
    use crate::session::types::SessionClaims;
    use crate::shared::test_utils::AppStateBuilder;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        response::IntoResponse,
        routing::get,
        Extension, Router,
    };
    use std::sync::Arc;
    use tower::ServiceExt; // for `oneshot`

    // Handler that requires authentication
    async fn protected_handler(Extension(claims): Extension<SessionClaims>) -> impl IntoResponse {
        (StatusCode::OK, format!("Hello, {}!", claims.username))
    }

    #[tokio::test]
    async fn test_jwt_middleware_allows_valid_token() {
        // Create app state with real session repository
        let session_repository = Arc::new(InMemorySessionRepository::new());
        let app_state = AppStateBuilder::new()
            .with_session_repository(session_repository)
            .build_with_test_defaults();

        // Create a valid session
        let session_response = app_state.session_service.create_session().await.unwrap();
        let valid_token = session_response.session_id;

        // Create router with middleware
        let app = Router::new()
            .route("/protected", get(protected_handler))
            .layer(middleware::from_fn_with_state(app_state.clone(), jwt_auth))
            .with_state(app_state);

        // Create request with valid Authorization header
        let request = Request::builder()
            .method("GET")
            .uri("/protected")
            .header("Authorization", format!("Bearer {}", valid_token))
            .body(Body::empty())
            .unwrap();

        // Call the handler
        let response = app.oneshot(request).await.unwrap();

        // Should succeed
        assert_eq!(response.status(), StatusCode::OK);

        // Verify response body contains username
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("Hello"));
        assert!(body_str.contains(&session_response.username));
    }

    #[tokio::test]
    async fn test_jwt_middleware_rejects_missing_header() {
        let app_state = AppStateBuilder::new().build_with_test_defaults();

        let app = Router::new()
            .route("/protected", get(protected_handler))
            .layer(middleware::from_fn_with_state(app_state.clone(), jwt_auth))
            .with_state(app_state);

        // Create request without Authorization header
        let request = Request::builder()
            .method("GET")
            .uri("/protected")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Should be unauthorized
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // Verify error message
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("authorization header"));
    }

    #[tokio::test]
    async fn test_jwt_middleware_rejects_invalid_token_format() {
        let app_state = AppStateBuilder::new().build_with_test_defaults();

        let app = Router::new()
            .route("/protected", get(protected_handler))
            .layer(middleware::from_fn_with_state(app_state.clone(), jwt_auth))
            .with_state(app_state);

        // Create request with malformed Authorization header (no "Bearer " prefix)
        let request = Request::builder()
            .method("GET")
            .uri("/protected")
            .header("Authorization", "InvalidToken123")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Should be unauthorized
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("authorization header format"));
    }

    #[tokio::test]
    async fn test_jwt_middleware_rejects_invalid_token() {
        let app_state = AppStateBuilder::new().build_with_test_defaults();

        let app = Router::new()
            .route("/protected", get(protected_handler))
            .layer(middleware::from_fn_with_state(app_state.clone(), jwt_auth))
            .with_state(app_state);

        // Create request with invalid token
        let request = Request::builder()
            .method("GET")
            .uri("/protected")
            .header("Authorization", "Bearer invalid.token.here")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Should be unauthorized
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_jwt_middleware_rejects_nonexistent_session() {
        let session_repository = Arc::new(InMemorySessionRepository::new());
        let app_state = AppStateBuilder::new()
            .with_session_repository(session_repository)
            .build_with_test_defaults();

        // Create and then delete a session
        let session_response = app_state.session_service.create_session().await.unwrap();
        let token = session_response.session_id.clone();
        app_state
            .session_service
            .revoke_session(&session_response.session_id)
            .await
            .unwrap();

        let app = Router::new()
            .route("/protected", get(protected_handler))
            .layer(middleware::from_fn_with_state(app_state.clone(), jwt_auth))
            .with_state(app_state);

        // Try to use the revoked token
        let request = Request::builder()
            .method("GET")
            .uri("/protected")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Should be unauthorized
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_jwt_middleware_extracts_claims_correctly() {
        let session_repository = Arc::new(InMemorySessionRepository::new());
        let app_state = AppStateBuilder::new()
            .with_session_repository(session_repository)
            .build_with_test_defaults();

        // Create session with known username
        let session_response = app_state.session_service.create_session().await.unwrap();
        let expected_username = session_response.username.clone();

        let app = Router::new()
            .route("/protected", get(protected_handler))
            .layer(middleware::from_fn_with_state(app_state.clone(), jwt_auth))
            .with_state(app_state);

        let request = Request::builder()
            .method("GET")
            .uri("/protected")
            .header(
                "Authorization",
                format!("Bearer {}", session_response.session_id),
            )
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify the username was extracted correctly
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains(&expected_username));
    }
}
