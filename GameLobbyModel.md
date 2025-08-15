# Big Two Game Data Model - Lobby Focus

## Overview

Minimal data model for lobby management with basic connection state tracking.

## Current Issues
- **❌ Broken Disconnect Handling**: Games become unplayable when players disconnect
- **❌ No Reconnection**: Players cannot rejoin active games after disconnect
- **❌ No Connection State**: No tracking of player connection status

## Core Entity

### GameLobby (Minimal)
**Purpose**: Track players in lobby with basic connection state
**Storage**: In-memory only

```rust
pub struct GameLobby {
    pub id: String,
    pub host: String,
    pub players: Vec<LobbyPlayer>,
}

pub struct LobbyPlayer {
    pub name: String,
    pub connection_status: ConnectionStatus,
}

pub enum ConnectionStatus {
    Connected,
    Disconnected,
    // TODO: Add Reconnecting state in future for grace period handling
}
```

## Connection Status Management

### Backend Responsibilities

#### WebSocket Connection Monitoring
- **Connection Detection**: Backend automatically detects WebSocket connection/disconnection events
- **Status Updates**: Backend updates `ConnectionStatus` in real-time based on WebSocket state
- **Event Broadcasting**: Backend notifies all room players when connection status changes

#### Connection Status Sources
```rust
// Backend automatically sets status based on:
Connected    -> WebSocket connection established and active
Disconnected -> WebSocket connection lost/closed
// Future: Reconnecting -> Player attempting to rejoin (grace period)
```

### Status Update Flow

#### 1. Player Disconnection (Automatic)
```
1. WebSocket connection lost detected by backend
2. Backend sets player status to Disconnected
3. Backend broadcasts PlayerStatusChanged event to room
4. Frontend receives update and shows "Player X disconnected"
```

#### 2. Player Reconnection (Automatic)
```
1. Player establishes new WebSocket connection
2. Backend validates session and room membership
3. Backend sets player status to Connected
4. Backend broadcasts PlayerStatusChanged event to room
5. Frontend receives update and shows "Player X reconnected"
```

#### 3. Simple Reconnection (Current Implementation)
```
1. Player reconnects with new WebSocket
2. Backend validates session and room membership
3. Player status immediately set to Connected
4. No grace period or intermediate states
```

#### 4. Future: Grace Period Handling (Potential Future Feature)
```
1. On disconnect, backend could start reconnection timer
2. Player status set to Reconnecting (future state)
3. If reconnection within timeout -> Connected
4. If timeout expires -> remove from lobby
```

## Expected WebSocket Events

### Backend → Frontend Events
```rust
// Player status change notification
PlayerStatusChanged {
    room_id: String,
    player_name: String,
    status: ConnectionStatus,  // Connected | Disconnected
    timestamp: DateTime,
}

// Room state update with connection info
RoomUpdated {
    room_id: String,
    players: Vec<LobbyPlayer>,  // includes connection_status for each
}
```

### Frontend → Backend Events
```rust
// Rejoin room after disconnect (implicit connection status update)
RejoinRoom {
    room_id: String,
    session_token: String,  // for validation
}
```

### Backend API Contracts

#### Connection Status Rules
- **Automatic Management**: Frontend should NOT manually set connection status
- **WebSocket Driven**: Status changes ONLY triggered by WebSocket events
- **Session Validation**: Reconnection requires valid session token
- **Room Membership**: Player must be in room to have connection status

#### Reconnection Validation
```rust
// Backend validates on reconnection:
1. Valid session token
2. Player exists in room
3. Room still active
4. Within reconnection timeout (if implemented)
```

## Migration Strategy

**Phase 1**: Add connection state to existing Room entity
- Rename `Room` -> `GameLobby` 
- Add `ConnectionStatus` to track player connections
- Implement basic disconnect/reconnect handling
- Add `PlayerStatusChanged` WebSocket event
- Update room state to include connection info