# Rust Migration Plan: Big Two Game Backend

## 📋 Current System Overview

The Python/FastAPI backend is a real-time multiplayer Big Two card game with the following components:

### Core Features
- **REST API** for room and session management
- **WebSocket connections** for real-time gameplay
- **Session management** with automatic cleanup
- **Room management** with host controls
- **Game state management** for Big Two card game
- **Card game logic** and move validation

### Pain Points Driving Migration
- **Limited Type Safety**: Python's dynamic typing makes refactoring risky
- **Testing Complexity**: Without comprehensive types, AI agents struggle to make safe changes
- **Runtime Errors**: Type-related issues only surface at runtime
- **Performance**: Python's GIL and interpretation overhead

## 🔌 REST API Endpoints

### Session Management

Users are not expected to have an account and can join any lobby. Doing so will assign them a session id in which if they disconnected, they'd be able to reconnect. This session id will persist for 7 days and extend each time they connect again.

| Endpoint | Method | Request | Response | Description |
|----------|--------|---------|----------|-------------|
| `/session/` | POST | None | `SessionResponse` | Create new session with auto-generated username |

### Room Management  
Creator of a room is also the host. If the host leaves the room then the next user is assigned as host. The host has the ability to "delete" the room.

| Endpoint | Method | Request | Response | Description |
|----------|--------|---------|----------|-------------|
| `/room/` | POST | `RoomCreateRequest` | `RoomResponse` | Create room with random ID |
| `/rooms/` | GET | None | `List[RoomResponse]` | Get all available rooms |
| `/rooms/{room_id}` | DELETE | Query: `host_name` | `RoomDeleteResponse` | Delete room (host only) |

### Utility Endpoints
| Endpoint | Method | Request | Response | Description |
|----------|--------|---------|----------|-------------|
| `/` | GET | None | `MessageResponse` | Health check / welcome message |

### WebSocket Endpoint
| Endpoint | Protocol | Description |
|----------|----------|-------------|
| `/ws/{room_id}` | WebSocket | Real-time game communication |

## 📡 WebSocket Communication Protocol

### Connection Flow
1. Client connects to `/ws/{room_id}` with optional query parameters
2. Server processes authentication via session validation
3. Server validates room exists and player can join
4. Connection established with initial messages sent to client

### Message Structure
All messages follow this JSON structure:
```json
{
  "type": "MESSAGE_TYPE",
  "payload": { /* message-specific data */ },
  "meta": {
    "timestamp": "2023-12-01T12:00:00Z",
    "player_id": "optional_player_id"
  }
}
```

### Message Types

#### Client → Server Messages
| Type | Payload Fields | Description |
|------|----------------|-------------|
| `CHAT` | `content: string` | Send chat message |
| `MOVE` | `cards: string[]` | Play cards |
| `LEAVE` | None | Leave the room |
| `START_GAME` | None | Start game (host only) |

#### Server → Client Messages
| Type | Payload Fields | Description |
|------|----------------|-------------|
| `PLAYERS_LIST` | `players: string[]` | Current players in room |
| `HOST_CHANGE` | `host: string` | New host assigned |
| `MOVE_PLAYED` | `player: string, cards: string[]` | Player played cards |
| `TURN_CHANGE` | `player: string` | Turn changed to player |
| `ERROR` | `message: string` | Error occurred |
| `GAME_STARTED` | `current_turn: string, cards: Card[]` | Game started with dealt cards |

## 🗄️ Data Storage

### Database Tables (PostgreSQL)

#### `rooms`
- `id` (String, Primary Key) - Random pet name generated ID
- `host_name` (String, Not Null) - Username of room host
- `status` (String) - "ONLINE" or "OFFLINE" 
- `player_count` (Integer) - Number of connected players

#### `user_sessions` 
- `id` (String, Primary Key) - UUID v4
- `username` (String, Not Null) - Auto-generated pet name
- `created_at` (DateTime) - Session creation time
- `expires_at` (DateTime) - Session expiration time

#### `users` (Currently unused)
- `id` (Integer, Primary Key)
- `username` (String, Unique)
- `password_hash` (String)
- `wins` (Integer)
- `losses` (Integer)

### In-Memory Storage

#### Game State Repository
- **Storage**: In-memory dictionary `Dict[room_id, GameStateModel]`
- **Interface**: `GameStateRepository` with methods:
  - `get_game(room_id)` → Optional game state
  - `set_game(room_id, game)` → Store game state
  - `delete_game(room_id)` → Remove game state
  - `has_game(room_id)` → Check if game exists

#### WebSocket Connection Manager
- **Storage**: In-memory nested dictionary `Dict[room_id, Dict[player_name, WebSocket]]`
- **Features**:
  - Track active WebSocket connections per room
  - Room-level locks for thread safety
  - Broadcast to all players in room
  - Send personal messages to specific players

## 🎮 Game State Management

### Game State Model
- `room_id`: String identifier
- `players`: List of players with hands
- `turn_index`: Current player's turn (0-3)
- `current_play`: Last played cards
- `played_hands`: History of all played hands

### Player Model
- `name`: Player username
- `hand`: List of cards in player's hand

### Game Operations
- **Add Player**: Add to game if under 4 players
- **Remove Player**: Remove and adjust turn index
- **Deal Cards**: Shuffle deck and deal 13 cards to each of 4 players
- **Play Cards**: Validate and remove cards from player's hand
- **Next Turn**: Advance to next player

## 🃏 Card System

### Card Representation
- **Suits**: Diamonds (1), Clubs (2), Hearts (3), Spades (4)
- **Ranks**: 3-10, J, Q, K, A, 2 (2 is highest in Big Two)
- **String Format**: "3D", "KH", "AS", etc.
- **Display Format**: "3♦", "K♥", "A♠", etc.

### Card Operations
- Parse string to Card object
- Convert Card to string representation
- Card comparison for game rules
- Deck creation (52 cards)
- Hand validation logic

## 🔧 Session Management

### Session Lifecycle
1. **Creation**: Auto-generates UUID and random username
2. **Validation**: Check session exists and hasn't expired
3. **Expiration**: 24-hour default lifespan
4. **Cleanup**: Background task removes expired sessions every 12 hours

### Session Features
- **Automatic Username Generation**: Uses petname library for readable names
- **Header-based Authentication**: `X-Session-ID` header for API calls
- **Query Parameter Auth**: Session ID in WebSocket connection URL
- **Background Cleanup**: Periodic removal of expired sessions

## 🏠 Room Management

### Room Lifecycle
1. **Creation**: Host creates room with auto-generated ID (petname)
2. **Joining**: Players connect via WebSocket to `/ws/{room_id}`
3. **Host Transfer**: If host leaves, ownership transfers to next player
4. **Deletion**: Host can delete room, or auto-deleted when empty

### Room Features
- **Random IDs**: Human-readable pet names (e.g., "happycat")
- **Player Tracking**: Real-time count in database
- **Host Controls**: Only host can start game or delete room
- **Status Tracking**: ONLINE/OFFLINE based on player connections

## 🔄 Connection Handling

### WebSocket Lifecycle
1. **Accept Connection**: Server accepts WebSocket connection
2. **Authentication**: Validate session from query parameters
3. **Room Validation**: Check room exists and player can join
4. **Message Loop**: Process incoming messages until disconnection
5. **Cleanup**: Remove from connection manager, update player count

### Reconnection Support
- Players can reconnect with same session ID
- Server sends current game state on reconnection
- Host status restored if reconnecting host

## 🏗️ Technology Migration

### Key Rust Advantages
- **Compile-time Type Checking**: Eliminate runtime type errors
- **Memory Safety**: No garbage collection overhead
- **Concurrency**: Fearless parallel processing with tokio
- **Performance**: 5-10x improvement expected in WebSocket throughput
- **Tooling**: Superior development experience for refactoring

### Compatibility Requirements
- **Same REST API**: All endpoints must have identical behavior
- **Same WebSocket Protocol**: Message format must remain unchanged
- **Database Schema**: Keep existing PostgreSQL tables
- **Session Format**: Maintain UUID-based sessions with same expiration

## 🎯 Migration Priorities

### Critical Features to Replicate
1. **Session Management**: UUID generation, expiration, cleanup
2. **Room Operations**: Creation, listing, deletion with host validation
3. **WebSocket Handling**: Connection management, message routing
4. **Game State**: In-memory storage with same data model
5. **Card System**: Exact same card representation and validation
6. **Error Handling**: Compatible error responses for frontend

### Non-Essential Features
- User table (currently unused)
- HTTPS certificate generation
- Complex deployment configurations

This migration will provide a more robust, performant, and maintainable backend while maintaining 100% compatibility with the existing TypeScript frontend. 