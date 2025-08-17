# WebSockets Module Documentation

## Overview

The `websockets` module provides real-time bidirectional communication for the Big Two application through WebSocket connections. It handles WebSocket upgrades, message routing, connection management, and integrates with the event system to provide real-time game updates. The module supports authentication via JWT tokens and manages per-player connections for room-based communication.

## Architecture

The websockets module follows a layered architecture with separation between connection handling, message processing, and event integration:

```
websockets/
├── mod.rs                      # Public API exports
├── handler.rs                  # WebSocket upgrade and connection management
├── connection_manager.rs       # Per-player connection tracking
├── socket.rs                   # Individual WebSocket connection wrapper
├── messages.rs                 # WebSocket message type definitions
└── websocket_room_subscriber.rs # Event handler for WebSocket broadcasts
```

## Key Components

### handler.rs - WebSocket Endpoint and Connection Management
**Responsibility**: Handles WebSocket upgrades, authentication, and connection lifecycle

**Key Features**:
- WebSocket upgrade endpoint: `GET /ws/{room_id}`
- JWT authentication via `Sec-WebSocket-Protocol` header
- Room existence validation
- Connection lifecycle management
- Initial state synchronization

**Strengths**:
- Clean separation of upgrade logic and connection handling
- Proper authentication integration
- Good error handling and logging
- Event-driven disconnect handling
- UUID-based player identity

**Potential Issues**:
- Authentication via header could be improved (standard Bearer tokens)
- No connection limits per player or room
- Limited error messages sent to client
- No connection health checks or ping/pong

### connection_manager.rs - Connection Tracking
**Responsibility**: Manages active WebSocket connections for message broadcasting

**Key Features**:
- UUID-based connection mapping
- Broadcast messaging to multiple players
- Automatic connection replacement on reconnect
- Thread-safe concurrent access

**Strengths**:
- Clean trait abstraction for different implementations
- Proper async/await usage
- Thread-safe with RwLock
- Simple API for message broadcasting

**Potential Issues**:
- No connection health monitoring
- Dropped messages if channel is full
- No connection metrics or statistics
- No cleanup of stale connections

### websocket_room_subscriber.rs - Event Handler
**Responsibility**: Processes room events and converts them to WebSocket messages

**Key Features**:
- Comprehensive event handling for all room/game events
- Message serialization and broadcasting
- Player state synchronization
- Error handling and recovery

**Event Handlers**:
- **Room Events**: PlayerJoined, PlayerLeft, HostChanged
- **Communication**: ChatMessage, PlayerLeaveRequested  
- **Game Events**: StartGame, MovePlayed, TurnChanged, GameWon, GameReset

**Strengths**:
- Comprehensive event coverage
- Good error handling and logging
- Clean separation of concerns
- Proper player UUID→name mapping

**Potential Issues**:
- Large file with many responsibilities (could split)
- Some code duplication in message broadcasting
- Error events not sent to clients
- No message ordering guarantees

### messages.rs - Message Type System
**Responsibility**: Defines WebSocket message formats and serialization

**Message Types**:
- **Client→Server**: Chat, Move, Leave, StartGame
- **Server→Client**: PlayersListed, HostChange, MovePlayed, TurnChange, GameStarted, GameWon, etc.

**Strengths**:
- Clean message type system
- Consistent JSON serialization
- Proper payload structures
- Good separation of concerns

**Potential Issues**:
- No message versioning
- Limited metadata (timestamps, IDs)
- No message acknowledgment system
- Basic error handling

### socket.rs - Connection Wrapper
**Responsibility**: Wraps individual WebSocket connections with message handling

**Key Features**:
- Bidirectional message handling
- Graceful connection cleanup
- Message handler integration
- Error handling and logging

**Strengths**:
- Clean abstraction over axum WebSocket
- Proper async message handling
- Good error recovery

**Potential Issues**:
- No message queuing or buffering
- Limited connection diagnostics
- No automatic reconnection support
- Basic backpressure handling

## Data Flow

### Connection Establishment
1. Client requests WebSocket upgrade to `/ws/{room_id}`
2. Server validates JWT token from `Sec-WebSocket-Protocol` header
3. Session service validates token and extracts player identity
4. Room service verifies room exists
5. Connection established and registered with ConnectionManager
6. Initial room state sent to client
7. Player connection event emitted

### Message Processing
1. **Inbound**: Client→Server via WebSocket
   - Socket receives message → WebsocketReceiveHandler
   - Message parsed and validated → Event emitted to EventBus
   - Event processed by game/room logic → State updated

2. **Outbound**: Server→Client via Events
   - Event emitted → WebSocketRoomSubscriber receives event
   - Event converted to WebSocket message → Sent via ConnectionManager
   - Message delivered to client connections

### Connection Termination
1. WebSocket connection closes (client disconnect or error)
2. Connection removed from ConnectionManager
3. PlayerDisconnected event emitted
4. Event system handles cleanup and notifications

## Message Protocol

### Authentication
- **Endpoint**: `GET /ws/{room_id}`
- **Auth Method**: JWT token in `Sec-WebSocket-Protocol` header
- **Format**: `Sec-WebSocket-Protocol: {jwt_token}`

### Message Format
```json
{
  "type": "MESSAGE_TYPE",
  "payload": { /* message-specific data */ },
  "meta": { /* optional metadata */ }
}
```

### Client→Server Messages
- `CHAT`: Send chat message to room
- `MOVE`: Play cards in game
- `LEAVE`: Leave room
- `START_GAME`: Start game (host only)

### Server→Client Messages
- `PLAYERS_LIST`: Updated player list
- `HOST_CHANGE`: New host assigned
- `MOVE_PLAYED`: Player made a move
- `TURN_CHANGE`: Turn advanced to next player
- `GAME_STARTED`: Game has begun
- `GAME_WON`: Game completed with winner
- `GAME_RESET`: Game returned to lobby

## Integration Points

### Event System Integration
- **Receives**: All room and game events for broadcasting
- **Emits**: Player connection/disconnection events
- **Processes**: Chat, move, and control messages from clients

### Service Dependencies
- **SessionService**: JWT validation and player identity
- **RoomService**: Room existence and state queries
- **GameService**: Game state for message context
- **PlayerMapping**: UUID↔name resolution

## Strengths

1. **Real-Time Communication**: Immediate game state updates
2. **Event Integration**: Clean separation via event system
3. **Authentication**: Proper JWT validation
4. **Error Handling**: Comprehensive error logging
5. **Scalable Design**: Connection manager abstraction
6. **Message Types**: Well-defined protocol

## Areas for Improvement

### Connection Management
- **Health Monitoring**: No ping/pong or connection health checks
- **Connection Limits**: No limits on connections per player/room
- **Stale Connections**: No automatic cleanup of dead connections
- **Reconnection**: No automatic reconnection support

### Message Handling
- **Message Ordering**: No guarantees on message delivery order
- **Acknowledgments**: No message delivery confirmation
- **Error Responses**: Limited error messages to clients
- **Message History**: No message persistence or replay

### Performance & Reliability
- **Backpressure**: Limited handling of slow clients
- **Message Queuing**: No message buffering or queuing
- **Metrics**: No connection or message metrics
- **Load Balancing**: No support for multiple server instances

### Security & Monitoring
- **Rate Limiting**: No protection against message flooding
- **Connection Validation**: Limited connection state validation
- **Audit Trail**: No logging of message history
- **Authorization**: No fine-grained permissions

## Recommended Improvements

### 1. Enhanced Connection Management
```rust
pub struct ConnectionMetrics {
    pub connected_at: SystemTime,
    pub last_message: SystemTime,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub is_healthy: bool,
}

impl ConnectionManager {
    async fn get_connection_metrics(&self, uuid: &str) -> Option<ConnectionMetrics>;
    async fn cleanup_stale_connections(&self, timeout: Duration) -> usize;
    async fn health_check_all(&self) -> Vec<String>; // Unhealthy UUIDs
}
```

### 2. Message Acknowledgment System
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub id: Option<String>,           // Message ID for acknowledgment
    pub message_type: MessageType,
    pub payload: serde_json::Value,
    pub timestamp: SystemTime,
    pub requires_ack: bool,
}

impl WebSocketMessage {
    pub fn ack(message_id: String) -> Self;
    pub fn error(code: u32, message: String) -> Self;
}
```

### 3. Enhanced Error Handling
```rust
#[derive(Debug, Serialize)]
pub enum WebSocketError {
    InvalidMessage { reason: String },
    GameError { code: String, message: String },
    RoomError { code: String, message: String },
    AuthenticationError { message: String },
}

impl WebSocketRoomSubscriber {
    async fn send_error_to_player(&self, uuid: &str, error: WebSocketError);
}
```

### 4. Connection Health Monitoring
```rust
pub struct HealthMonitor {
    ping_interval: Duration,
    pong_timeout: Duration,
}

impl HealthMonitor {
    pub async fn start_monitoring(&self, connections: Arc<dyn ConnectionManager>);
    pub async fn ping_all_connections(&self);
    pub async fn handle_pong(&self, uuid: &str);
}
```

## Critical Issues to Address

1. **Memory Leaks**: Connections not cleaned up on abnormal disconnect
2. **Security Gaps**: No rate limiting or message validation
3. **Single Point of Failure**: In-memory connection manager not distributed
4. **Performance Issues**: No backpressure handling for slow clients

## Testing Considerations

The module would benefit from:
- WebSocket integration tests with real connections
- Load testing for concurrent connections
- Failure scenario testing (network issues, etc.)
- Message ordering and delivery testing
- Authentication bypass testing

## Performance Considerations

- **Memory Usage**: Connection storage grows with player count
- **CPU Usage**: JSON serialization on every message
- **Network**: No message compression or optimization
- **Concurrency**: RwLock contention with many connections

## Security Considerations

- **Authentication**: JWT validation but no refresh
- **Authorization**: No per-message permissions
- **Rate Limiting**: No protection against abuse
- **Message Validation**: Basic validation only

## Overall Assessment

The websockets module provides a solid foundation for real-time communication with good event system integration and clean architecture. The message protocol is well-defined and the connection management is adequate for basic use. However, it lacks production features like health monitoring, message acknowledgments, and comprehensive error handling. The module would benefit from enhanced reliability and monitoring capabilities.

**Real-Time Communication**: 8/10
**Maintainability**: 7/10
**Performance**: 6/10
**Reliability**: 5/10
**Security**: 6/10
**Overall**: 7/10