use async_trait::async_trait;
use thiserror::Error;

use super::events::GameEvent;

/// Errors that can occur when handling events
#[derive(Debug, Error)]
pub enum EventError {
    #[error("Handler timed out")]
    Timeout,

    #[error("Retryable error: {0}")]
    Retryable(String),

    #[error("Non-retryable error: {0}")]
    NonRetryable(String),

    #[error("Handler panicked: {0}")]
    Panic(String),
}

impl EventError {
    /// Whether this error indicates the operation should be retried
    pub fn is_retryable(&self) -> bool {
        matches!(self, EventError::Retryable(_) | EventError::Timeout)
    }

    /// Create a retryable error
    pub fn retryable(msg: impl Into<String>) -> Self {
        EventError::Retryable(msg.into())
    }

    /// Create a non-retryable error
    pub fn non_retryable(msg: impl Into<String>) -> Self {
        EventError::NonRetryable(msg.into())
    }
}

/// Trait for components that can handle game events
///
/// Event handlers are the reactive components in our system.
/// They listen for specific events and perform actions in response.
///
/// Examples:
/// - ConnectionEventHandler: broadcasts updates to WebSocket connections
/// - DatabaseEventHandler: writes game data to the database
/// - AnalyticsEventHandler: records metrics and statistics
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Handle a game event
    ///
    /// This method should:
    /// - Check if the event is relevant to this handler
    /// - Perform the appropriate action
    /// - Return Ok(()) on success or EventError on failure
    ///
    /// Handlers should be idempotent where possible - handling the same
    /// event multiple times should be safe.
    async fn handle(&self, event: &GameEvent) -> Result<(), EventError>;

    /// Get a human-readable name for this handler (for logging/debugging)
    fn name(&self) -> &'static str;
}

/// A no-op event handler for testing
///
/// This handler does nothing but can be used in tests where you need
/// an EventHandler but don't care about the actual behavior.
pub struct NoOpEventHandler;

#[async_trait]
impl EventHandler for NoOpEventHandler {
    async fn handle(&self, _event: &GameEvent) -> Result<(), EventError> {
        Ok(())
    }

    fn name(&self) -> &'static str {
        "NoOpEventHandler"
    }
}
