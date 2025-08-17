# Room Module Documentation

## Overview

The `room` module manages the lifecycle of game rooms in the Big Two application. It handles room creation, player joining/leaving, and maintains room state through a clean service layer architecture. The module provides both HTTP API endpoints and internal services for room management with support for pet-name room IDs and UUID-based player tracking.

## Architecture

The room module follows a layered architecture with clear separation between HTTP handlers, business logic, and data persistence:

```
room/
├── mod.rs           # Public API exports
├── handlers.rs      # HTTP endpoint handlers
├── service.rs       # Business logic layer
├── repository.rs    # Data persistence abstractions
├── models.rs        # Data structures
└── types.rs         # Request/response types
```

## Key Components

### handlers.rs - HTTP API Layer
**Responsibility**: Exposes REST endpoints for room operations

**Endpoints**:
- `POST /room` - Create new room (authenticated)
- `GET /rooms` - List all rooms (public)  
- `POST /room/{id}/join` - Join room (authenticated)
- `GET /room/{id}` - Get room details (public)

**Strengths**:
- Clean separation of HTTP concerns from business logic
- Proper authentication integration with session middleware
- Good error handling and logging
- Event-driven integration for real-time updates
- Automatic WebSocket subscription setup

**Potential Issues**:
- WebSocket subscription handles not stored for cleanup
- Event emissions could be moved to service layer
- No rate limiting on room creation
- Limited input validation

### service.rs - Business Logic Layer
**Responsibility**: Orchestrates room operations and coordinates between repository and player mapping

**Key Features**:
- Room creation with pet-name ID generation
- Player joining with capacity limits (4 players)
- UUID-to-display-name mapping
- Atomic operations for concurrent access

**Strengths**:
- Clean service layer pattern
- Good separation of concerns
- Proper error handling and logging
- Comprehensive test coverage
- UUID-based player identity

**Potential Issues**:
- No room cleanup mechanism
- Hard-coded capacity limits
- Player mapping dependency could be better abstracted
- No room state validation

### repository.rs - Data Persistence Layer
**Responsibility**: Manages room data storage with atomic operations

**Key Types**:
- `RoomRepository`: Async trait for storage operations
- `InMemoryRoomRepository`: In-memory implementation
- `JoinRoomResult`/`LeaveRoomResult`: Operation result types

**Strengths**:
- Clean repository pattern with trait abstraction
- Atomic operations prevent race conditions
- Comprehensive result types for different scenarios
- Good test coverage for edge cases
- Proper mutex usage for thread safety

**Potential Issues**:
- Only in-memory implementation (no persistence)
- No cleanup of inactive rooms
- Limited to 4-player capacity (hard-coded)
- No room history or audit trail

### models.rs - Data Structures
**Responsibility**: Defines core room data structure

**Key Features**:
- Pet-name ID generation using `petname` crate
- Player tracking with UUIDs
- Capacity management and validation
- Host transfer on player departure

**Strengths**:
- Clean data model with proper encapsulation
- Readable room IDs (e.g., "happy-brown-dog")
- Good helper methods for common operations

**Potential Issues**:
- Host logic could be more sophisticated
- No room metadata (creation time, game state)
- Player list could use HashSet for faster lookups

## Data Flow

### Room Creation
1. `POST /room` → `create_room` handler
2. Extract host UUID from authenticated session
3. `RoomService.create_room()` → create `RoomModel` with generated ID
4. Store in repository → Return response with mapped display name
5. Start WebSocket subscription for real-time events

### Room Joining
1. `POST /room/{id}/join` → `join_room` handler
2. Extract player UUID from authenticated session
3. `RoomService.join_room()` → atomic `try_join_room()`
4. Check capacity and add player → Emit `PlayerJoined` event
5. Return updated room state

### Player Management
- **Host Transfer**: Automatic when host leaves
- **Room Deletion**: Automatic when last player leaves
- **Capacity Control**: Maximum 4 players (Big Two requirement)

## Room States and Lifecycle

### Room States
- **ONLINE**: Active room accepting players
- **IN_GAME**: Game in progress (future enhancement)
- **DELETED**: Room removed after emptying

### Lifecycle Events
1. **Creation**: Host creates room → WebSocket subscription starts
2. **Population**: Players join → Events broadcast to subscribers
3. **Game Start**: When 4 players present (handled by game module)
4. **Completion**: Game ends → Room can reset or delete
5. **Cleanup**: Last player leaves → Room auto-deleted

## Player Identity System

The module uses a dual identity system:
- **UUIDs**: Internal stable identity from sessions
- **Display Names**: Human-readable names from player mapping service

This allows for stable identity tracking while providing flexible display name changes.

## Strengths

1. **Clean Architecture**: Well-separated concerns between layers
2. **Atomic Operations**: Thread-safe concurrent access
3. **Event Integration**: Real-time updates through event system
4. **Good Testing**: Comprehensive test coverage
5. **Pet-Name IDs**: User-friendly room identifiers
6. **Session Integration**: Proper authentication flow

## Areas for Improvement

### Resource Management
- **Room Cleanup**: No mechanism to remove inactive rooms
- **Subscription Cleanup**: WebSocket subscription handles not stored
- **Memory Leaks**: Rooms persist indefinitely until empty
- **Resource Limits**: No global limits on room creation

### Configuration
- **Hard-Coded Limits**: 4-player capacity not configurable
- **Fixed IDs**: Pet-name generation not customizable
- **Timeout Settings**: No configurable timeouts
- **Capacity Management**: No server-wide room limits

### Error Handling
- **Error Granularity**: Limited error types for different scenarios
- **Recovery Mechanisms**: No retry logic for transient failures
- **Validation**: Limited input validation
- **Monitoring**: No metrics on room operations

### Features
- **Room Metadata**: No creation time, last activity tracking
- **Room Modes**: No support for different game modes
- **Persistence**: Only in-memory storage available
- **Admin Controls**: No administrative room management

## Recommended Improvements

### 1. Resource Management
```rust
pub struct RoomConfig {
    pub max_rooms: usize,
    pub max_players_per_room: usize,
    pub inactive_room_timeout: Duration,
    pub cleanup_interval: Duration,
}

impl RoomService {
    pub async fn cleanup_inactive_rooms(&self) -> usize { /* ... */ }
    pub async fn get_room_metrics(&self) -> RoomMetrics { /* ... */ }
}
```

### 2. Enhanced Room Model
```rust
#[derive(Debug, Clone)]
pub struct RoomModel {
    pub id: String,
    pub host_uuid: Option<String>,
    pub status: RoomStatus,
    pub player_uuids: Vec<String>,
    pub created_at: SystemTime,
    pub last_activity: SystemTime,
    pub game_mode: GameMode,
    pub max_players: usize,
}
```

### 3. Persistence Layer
```rust
pub struct PostgresRoomRepository {
    pool: PgPool,
}

#[async_trait]
impl RoomRepository for PostgresRoomRepository {
    // Implement with proper database persistence
}
```

### 4. Enhanced Error Types
```rust
#[derive(Debug, Error)]
pub enum RoomServiceError {
    #[error("Room not found: {0}")]
    RoomNotFound(String),
    #[error("Room capacity exceeded: {current}/{max}")]
    RoomFull { current: usize, max: usize },
    #[error("Player already in room: {0}")]
    PlayerAlreadyInRoom(String),
    #[error("Server capacity exceeded: {0} rooms")]
    ServerCapacityExceeded(usize),
}
```

## Critical Issues to Address

1. **Memory Leaks**: Rooms never cleaned up automatically
2. **Resource Exhaustion**: No limits on room creation
3. **Subscription Leaks**: WebSocket subscription handles not managed
4. **No Persistence**: Only in-memory storage available

## Testing Considerations

The module has good test coverage but could benefit from:
- Load testing for concurrent operations
- Integration tests with real WebSocket connections
- Performance tests for large numbers of rooms
- Failure scenario testing (network issues, etc.)

## Integration Points

### Dependencies
- **Session Module**: For player authentication and UUID resolution
- **Event Module**: For real-time event broadcasting
- **WebSocket Module**: For subscription setup
- **Player Mapping**: For UUID-to-name translation

### Event Emissions
- `PlayerJoined`: When player successfully joins
- `PlayerLeft`: When player leaves (future)
- `HostChanged`: When host changes (future)
- `RoomDeleted`: When room is removed (future)

## Overall Assessment

The room module provides a solid foundation for room management with clean architecture and good separation of concerns. The atomic operations and event integration are well-designed. However, it lacks production features like persistence, resource management, and cleanup mechanisms. The module would benefit from configuration options and enhanced error handling.

**Maintainability Score**: 8/10
**Scalability**: 6/10
**Feature Completeness**: 7/10
**Production Readiness**: 5/10
**Overall**: 7/10