# Event Module Documentation

## Overview

The `event` module provides a lightweight, room-based event system that enables decoupled communication between different components of the Big Two application. It implements the Observer pattern using Tokio's broadcast channels to allow multiple subscribers to react to room-specific events without tight coupling.

## Architecture

The event system follows a hub-and-spoke pattern with the EventBus as the central coordinator:

```
event/
├── mod.rs                # Public API exports
├── bus.rs                # EventBus - central event coordinator
├── events.rs             # RoomEvent definitions
├── room_handler.rs       # Handler trait and error types
└── room_subscription.rs  # Subscription management and event routing
```

## Key Components

### bus.rs - EventBus Implementation
**Responsibility**: Central event coordination and room-specific channel management

**Key Features**:
- Room-specific broadcast channels (one per room)
- Automatic channel creation on first emit/subscribe
- Thread-safe with Arc<RwLock> for concurrent access
- Tokio broadcast channels for efficient fan-out

**Implementation Details**:
```rust
pub struct EventBus {
    room_channels: Arc<RwLock<HashMap<String, broadcast::Sender<RoomEvent>>>>
}
```

**Strengths**:
- Clean separation of concerns (room isolation)
- Efficient broadcast to multiple subscribers
- Automatic channel management
- Good error handling and logging

**Potential Issues**:
- No channel cleanup (memory leak for inactive rooms)
- Fixed channel capacity (100 messages) could overflow
- No backpressure handling for slow consumers
- Channel creation race condition possible

### events.rs - Event Definitions
**Responsibility**: Defines all room-specific events in the system

**Event Categories**:
1. **Room Management**: PlayerJoined, PlayerLeft, HostChanged
2. **Communication**: ChatMessage, PlayerConnected, PlayerDisconnected  
3. **Game Flow**: TryStartGame, CreateGame, StartGame, GameReset
4. **Gameplay**: TryPlayMove, MovePlayed, TurnChanged, GameWon

**Strengths**:
- Comprehensive event coverage
- Clear naming convention (Try* for attempts, past tense for completed)
- Serializable for potential network distribution
- Well-structured with relevant data

**Potential Issues**:
- Large enum could impact compile times
- No event versioning or backward compatibility
- Missing metadata (timestamp, correlation ID)
- No event prioritization

### room_handler.rs - Handler Interface
**Responsibility**: Defines the contract for event handlers

**Key Types**:
- `RoomEventHandler`: Async trait for processing events
- `RoomEventError`: Comprehensive error types

**Strengths**:
- Clean async trait interface
- Good error categorization
- Handler naming for debugging
- Flexible error handling

**Potential Issues**:
- No retry mechanism built in
- No timeout handling
- Error types could be more specific
- No metrics/monitoring hooks

### room_subscription.rs - Subscription Management
**Responsibility**: Manages long-running event subscriptions and routes events to handlers

**Key Features**:
- Background task spawning for each subscription
- Automatic error handling and logging
- Clean subscription lifecycle management

**Strengths**:
- Good separation of concerns
- Robust error handling
- Comprehensive logging
- Clean async task management

**Potential Issues**:
- No subscription cleanup mechanism
- Memory leak potential from abandoned tasks
- No metrics on handler performance
- No circuit breaker for failing handlers

## Data Flow

1. **Event Creation**: Component calls `event_bus.emit_to_room(room_id, event)`
2. **Channel Routing**: EventBus routes to room-specific broadcast channel
3. **Event Distribution**: All room subscribers receive the event
4. **Handler Processing**: Each handler processes the event independently
5. **Error Handling**: Handler errors are logged but don't affect other handlers

## Event Types and Usage

### Room Lifecycle Events
```rust
PlayerJoined { player: String }     // User joins room
PlayerLeft { player: String }       // User leaves room  
HostChanged { old_host, new_host }  // Host transfer
```

### Game Flow Events
```rust
TryStartGame { host }               // Host attempts to start
CreateGame { players }              // Game creation confirmed
StartGame { game }                  // Game fully initialized
GameReset                          // Game returns to lobby
```

### Gameplay Events
```rust
TryPlayMove { player, cards }       // Move attempt
MovePlayed { player, cards, game }  // Valid move executed
TurnChanged { player }             // Turn advancement
GameWon { winner }                 // Game completion
```

## Subscribers

### Current Implementations
1. **GameEventRoomSubscriber**: Processes game logic events
2. **WebSocketRoomSubscriber**: Broadcasts events to WebSocket connections

### Subscription Pattern
```rust
let subscription = RoomSubscription::new(
    room_id.to_string(),
    Arc::new(handler),
    event_bus.clone(),
);
subscription.start().await;
```

## Strengths

1. **Decoupling**: Components can communicate without direct dependencies
2. **Scalability**: Multiple handlers can process events independently
3. **Room Isolation**: Events are scoped to specific rooms
4. **Async Design**: Non-blocking event processing
5. **Error Isolation**: Handler failures don't affect other handlers

## Areas for Improvement

### Resource Management
- **Channel Cleanup**: No mechanism to remove inactive room channels
- **Subscription Lifecycle**: Abandoned subscriptions leak memory
- **Backpressure**: No handling of slow consumers
- **Resource Limits**: No limits on subscription count

### Error Handling
- **Retry Logic**: No automatic retry for transient failures
- **Circuit Breaker**: No protection against consistently failing handlers
- **Dead Letter Queue**: No handling of persistently failed events
- **Timeout Handling**: No timeouts for slow handlers

### Observability
- **Metrics**: No performance monitoring or handler metrics
- **Tracing**: Limited correlation between events and handlers
- **Health Checks**: No way to verify handler health
- **Event History**: No audit trail of events

### Configuration
- **Channel Capacity**: Hard-coded 100 message limit
- **Handler Timeouts**: No configurable timeouts
- **Retry Policies**: No configurable retry behavior
- **Priority Queues**: No event prioritization

## Recommended Improvements

### 1. Resource Management
```rust
pub struct EventBusConfig {
    pub channel_capacity: usize,
    pub cleanup_interval: Duration,
    pub max_subscriptions_per_room: usize,
}

impl EventBus {
    pub async fn cleanup_inactive_rooms(&self) -> usize { /* ... */ }
    pub async fn get_room_stats(&self) -> HashMap<String, RoomStats> { /* ... */ }
}
```

### 2. Enhanced Error Handling
```rust
pub struct HandlerConfig {
    pub max_retries: u32,
    pub timeout: Duration,
    pub circuit_breaker_threshold: u32,
}

#[async_trait]
pub trait RoomEventHandler {
    async fn handle_room_event_with_retry(
        &self,
        room_id: &str,
        event: RoomEvent,
        config: &HandlerConfig,
    ) -> Result<(), RoomEventError>;
}
```

### 3. Event Metadata
```rust
#[derive(Debug, Clone)]
pub struct EventEnvelope {
    pub event: RoomEvent,
    pub timestamp: SystemTime,
    pub correlation_id: String,
    pub retry_count: u32,
}
```

### 4. Metrics Integration
```rust
pub trait EventMetrics {
    fn record_event_emitted(&self, room_id: &str, event_type: &str);
    fn record_handler_duration(&self, handler: &str, duration: Duration);
    fn record_handler_error(&self, handler: &str, error_type: &str);
}
```

## Critical Issues to Address

1. **Memory Leaks**: Room channels and subscriptions never cleaned up
2. **Backpressure**: Slow handlers can cause channel overflow
3. **Error Recovery**: No mechanism to recover from handler failures
4. **Resource Exhaustion**: No limits on concurrent subscriptions

## Testing Considerations

The event system needs comprehensive testing for:
- Concurrent event emission and handling
- Handler failure scenarios
- Channel overflow conditions
- Subscription lifecycle management
- Memory usage under load

## Overall Assessment

The event module provides a solid foundation for decoupled communication with good separation of concerns and clean async design. However, it lacks production-ready features like resource management, comprehensive error handling, and observability. The design is extensible but needs significant hardening for production use.

**Maintainability Score**: 8/10
**Scalability**: 6/10
**Error Handling**: 5/10
**Resource Management**: 3/10
**Overall**: 6/10