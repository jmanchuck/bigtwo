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
