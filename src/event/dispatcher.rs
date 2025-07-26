use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use super::{
    bus::EventBus,
    events::GameEvent,
    handler::{EventError, EventHandler},
};

/// Coordinates event distribution between the event bus and event handlers
///
/// The EventDispatcher is the "town crier's assistant" that:
/// - Listens for events from the EventBus
/// - Routes events to the appropriate handlers
/// - Handles retries and error recovery
/// - Provides isolation between handlers (one failing handler doesn't affect others)
pub struct EventDispatcher {
    handlers: Vec<Arc<dyn EventHandler>>,
    event_bus: EventBus,
    handler_timeout: Duration,
    max_retries: u32,
}

impl EventDispatcher {
    /// Create a new event dispatcher
    pub fn new(event_bus: EventBus) -> Self {
        Self {
            handlers: Vec::new(),
            event_bus,
            handler_timeout: Duration::from_secs(5),
            max_retries: 3,
        }
    }

    /// Add an event handler to the dispatcher
    ///
    /// The handler will start receiving events once `start_listening` is called.
    pub fn add_handler(&mut self, handler: Arc<dyn EventHandler>) {
        info!(handler_name = handler.name(), "Registering event handler");
        self.handlers.push(handler);
    }

    /// Set the timeout for individual handler execution
    pub fn with_handler_timeout(mut self, timeout: Duration) -> Self {
        self.handler_timeout = timeout;
        self
    }

    /// Set the maximum number of retries for failed handlers
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Start listening for events and dispatching them to handlers
    ///
    /// This spawns a background task that will run until the EventBus
    /// is dropped or all handlers are removed.
    pub async fn start_listening(self) {
        let handlers = self.handlers;
        let mut receiver = self.event_bus.subscribe();
        let handler_timeout = self.handler_timeout;
        let max_retries = self.max_retries;

        info!(
            handler_count = handlers.len(),
            timeout_secs = handler_timeout.as_secs(),
            max_retries = max_retries,
            "Starting event dispatcher"
        );

        tokio::spawn(async move {
            while let Ok(event) = receiver.recv().await {
                debug!(
                    event_type = event.event_type(),
                    room_id = event.room_id(),
                    "Dispatching event to {} handlers",
                    handlers.len()
                );

                // Process each handler independently
                for handler in &handlers {
                    let event = event.clone();
                    let handler = handler.clone();
                    let timeout_duration = handler_timeout;

                    // Spawn each handler in its own task for isolation
                    tokio::spawn(async move {
                        if let Err(e) =
                            Self::handle_with_retry(handler, event, timeout_duration, max_retries)
                                .await
                        {
                            error!(error = ?e, "Handler failed permanently");
                        }
                    });
                }
            }

            info!("Event dispatcher stopped listening");
        });
    }

    /// Handle an event with retry logic and timeout
    async fn handle_with_retry(
        handler: Arc<dyn EventHandler>,
        event: GameEvent,
        handler_timeout: Duration,
        max_retries: u32,
    ) -> Result<(), EventError> {
        let handler_name = handler.name();
        let event_type = event.event_type();
        let room_id = event.room_id();

        for attempt in 0..=max_retries {
            match timeout(handler_timeout, handler.handle(&event)).await {
                Ok(Ok(())) => {
                    if attempt > 0 {
                        info!(
                            handler = handler_name,
                            event_type = event_type,
                            room_id = room_id,
                            attempt = attempt + 1,
                            "Handler succeeded after retry"
                        );
                    }
                    return Ok(());
                }
                Ok(Err(e)) if e.is_retryable() && attempt < max_retries => {
                    warn!(
                        handler = handler_name,
                        event_type = event_type,
                        room_id = room_id,
                        attempt = attempt + 1,
                        error = ?e,
                        "Handler failed, will retry"
                    );

                    // Exponential backoff
                    let delay = Duration::from_millis(100 * 2_u64.pow(attempt));
                    tokio::time::sleep(delay).await;
                }
                Ok(Err(e)) => {
                    error!(
                        handler = handler_name,
                        event_type = event_type,
                        room_id = room_id,
                        attempt = attempt + 1,
                        error = ?e,
                        "Handler failed permanently"
                    );
                    return Err(e);
                }
                Err(_timeout) => {
                    let timeout_error = EventError::Timeout;
                    if attempt < max_retries {
                        warn!(
                            handler = handler_name,
                            event_type = event_type,
                            room_id = room_id,
                            attempt = attempt + 1,
                            "Handler timed out, will retry"
                        );
                        continue;
                    } else {
                        error!(
                            handler = handler_name,
                            event_type = event_type,
                            room_id = room_id,
                            "Handler timed out permanently"
                        );
                        return Err(timeout_error);
                    }
                }
            }
        }

        unreachable!("Loop should have returned by now");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering};
    use tokio::time::{sleep, Duration};

    struct CountingHandler {
        name: &'static str,
        call_count: AtomicU32,
    }

    impl CountingHandler {
        fn new(name: &'static str) -> Arc<Self> {
            Arc::new(Self {
                name,
                call_count: AtomicU32::new(0),
            })
        }

        fn call_count(&self) -> u32 {
            self.call_count.load(Ordering::Relaxed)
        }
    }

    #[async_trait]
    impl EventHandler for CountingHandler {
        async fn handle(&self, _event: &GameEvent) -> Result<(), EventError> {
            self.call_count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        fn name(&self) -> &'static str {
            self.name
        }
    }

    #[tokio::test]
    async fn test_dispatcher_basic_functionality() {
        let event_bus = EventBus::with_default_capacity();
        let mut dispatcher = EventDispatcher::new(event_bus.clone());

        let handler1 = CountingHandler::new("handler1");
        let handler2 = CountingHandler::new("handler2");

        dispatcher.add_handler(handler1.clone());
        dispatcher.add_handler(handler2.clone());

        // Start the dispatcher
        dispatcher.start_listening().await;

        // Give it a moment to start
        sleep(Duration::from_millis(10)).await;

        // Emit an event
        let event = GameEvent::LobbyCreated {
            room_id: "test-room".to_string(),
            host: "Alice".to_string(),
        };

        event_bus.emit(event);

        // Give handlers time to process
        sleep(Duration::from_millis(50)).await;

        // Both handlers should have been called
        assert_eq!(handler1.call_count(), 1);
        assert_eq!(handler2.call_count(), 1);
    }

    struct FailingHandler {
        fail_count: AtomicU32,
        max_failures: u32,
    }

    impl FailingHandler {
        fn new(max_failures: u32) -> Arc<Self> {
            Arc::new(Self {
                fail_count: AtomicU32::new(0),
                max_failures,
            })
        }
    }

    #[async_trait]
    impl EventHandler for FailingHandler {
        async fn handle(&self, _event: &GameEvent) -> Result<(), EventError> {
            let current = self.fail_count.fetch_add(1, Ordering::Relaxed);
            if current < self.max_failures {
                Err(EventError::retryable("Simulated failure"))
            } else {
                Ok(())
            }
        }

        fn name(&self) -> &'static str {
            "FailingHandler"
        }
    }

    #[tokio::test]
    async fn test_dispatcher_retry_logic() {
        let event_bus = EventBus::with_default_capacity();
        let mut dispatcher = EventDispatcher::new(event_bus.clone())
            .with_max_retries(3)
            .with_handler_timeout(Duration::from_millis(100));

        // Handler that fails twice then succeeds
        let handler = FailingHandler::new(2);
        dispatcher.add_handler(handler.clone());

        dispatcher.start_listening().await;
        sleep(Duration::from_millis(10)).await;

        let event = GameEvent::LobbyCreated {
            room_id: "test-room".to_string(),
            host: "Alice".to_string(),
        };

        event_bus.emit(event);

        // Give enough time for retries
        sleep(Duration::from_millis(1000)).await;

        // Should have been called 3 times (initial + 2 retries)
        assert_eq!(handler.fail_count.load(Ordering::Relaxed), 3);
    }
}
