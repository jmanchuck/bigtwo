use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use tracing::{info, instrument, warn};

use super::service::SessionService;
use crate::shared::{AppError, AppState};

/// JWT authentication middleware - validates X-Session-ID header and adds SessionClaims to request.
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

    // Use injected repository from app state
    let service = SessionService::new(Arc::clone(&state.session_repository));

    // Extract token from X-Session-ID header
    let token = req
        .headers()
        .get("X-Session-ID")
        .and_then(|header| header.to_str().ok())
        .ok_or_else(|| {
            warn!("Missing X-Session-ID header in request");
            AppError::Unauthorized("Missing session token".to_string())
        })?;

    info!("Extracted token from X-Session-ID header");

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
