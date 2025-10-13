# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is the Rust backend for Big Two, a real-time multiplayer card game. Built with axum + tokio, it features event-driven architecture, WebSocket communication, and complete Big Two gameplay mechanics.

## Development Commands

```bash
# Development (auto-detects PostgreSQL or falls back to in-memory)
./scripts/dev.sh --postgres    # Force PostgreSQL
./scripts/dev.sh --memory      # Force in-memory storage

# Build and test
cargo check                    # Fast compile check
cargo test                     # Unit tests
cargo test -- --ignored       # Integration tests
cargo clippy                   # Lint
cargo fmt                      # Format

# Database (when using PostgreSQL)
sqlx migrate run              # Apply migrations
sqlx migrate add <name>       # Create migration

# Manual testing
./scripts/test-session.sh     # Test REST endpoints
```

## Architecture Overview

**Event-Driven Architecture**: Uses EventBus for decoupled communication between components. Game logic and WebSocket handling are separated into different event subscribers.

**Repository Pattern**: Abstractions for data storage with in-memory and PostgreSQL implementations.

**Smart Configuration**: Automatically uses PostgreSQL if `DATABASE_URL` is set, otherwise falls back to in-memory storage.

## Directory Structure

```
src/
├── main.rs                   # Entry point, axum server setup, dependency injection
├── lib.rs                    # Public API for integration tests
├── shared.rs                 # AppState, AppError, test utilities
├── event/                    # Event system for decoupled communication
│   ├── bus.rs               # EventBus implementation
│   ├── events.rs            # RoomEvent definitions
│   ├── room_handler.rs      # Event handler trait
│   └── room_subscription.rs # Room-specific event subscriptions
├── session/                  # JWT-based session management (7-day expiry)
│   ├── handlers.rs          # REST endpoints: create, validate
│   ├── middleware.rs        # JWT authentication middleware
│   ├── repository.rs        # In-memory + PostgreSQL implementations
│   ├── service.rs           # Business logic
│   ├── token.rs             # JWT utilities
│   └── models.rs, types.rs  # Data structures
├── room/                     # Game room lifecycle management
│   ├── handlers.rs          # REST endpoints: create, join, list, get
│   ├── repository.rs        # In-memory storage (uses pet-name IDs)
│   ├── service.rs           # Business logic
│   └── models.rs, types.rs  # Data structures
├── game/                     # Big Two game logic and state
│   ├── cards/               # Card system
│   │   ├── basic.rs        # Card types, Big Two sorting rules
│   │   └── hands.rs        # Hand validation and comparison
│   ├── core.rs              # Core game rules, turn progression
│   ├── repository.rs        # Game state repository
│   ├── service.rs           # Game service layer
│   └── game_room_subscriber.rs # Event handler for game logic
├── websockets/               # Real-time WebSocket communication
│   ├── handler.rs           # WebSocket upgrade and message routing
│   ├── messages.rs          # Message type definitions
│   ├── event_handlers/      # Organized event handling
│   │   ├── chat_events.rs  # Chat message handling
│   │   ├── game_events.rs  # Game move handling
│   │   ├── room_events.rs  # Room lifecycle events
│   │   ├── connection_events.rs # Connection/disconnection
│   │   └── shared/         # Shared utilities (player mapping, broadcasts)
│   ├── connection_manager.rs # Per-room connection tracking
│   ├── socket.rs            # Individual WebSocket handling
│   └── websocket_room_subscriber.rs # Event handler for WebSocket broadcast
├── bot/                      # AI bot system
│   ├── manager.rs           # Bot lifecycle management
│   ├── basic_strategy.rs    # Basic bot playing strategy
│   ├── bot_room_subscriber.rs # Bot event handling
│   ├── handlers.rs          # REST endpoints for bot operations
│   └── types.rs             # Bot-related types
└── user/                     # User management
    └── mapping_service.rs   # Player ID to username mapping
```

## Key Components

### AppState (shared.rs)
Central dependency injection container holding all repositories, services, managers, and the event bus. Contains builder pattern for testing.

### EventBus (event/)
Central message broker enabling decoupled communication. Supports both global and room-specific event subscriptions. Key event types include game moves, player connections/disconnections, and room lifecycle events.

### GameService (game/service.rs)
Manages Big Two game state per room. Handles game creation, move validation, turn progression, and win detection. Uses event system for communication.

### ConnectionManager (websockets/connection_manager.rs)
Tracks WebSocket connections per room for message broadcasting. Manages connection lifecycle and message routing.

### BotManager (bot/manager.rs)
Manages AI bot players in rooms. Handles bot creation, move generation, and lifecycle. Bots use basic strategy to play valid moves.

### Repository Pattern
- **SessionRepository**: JWT session storage (in-memory or PostgreSQL)
- **RoomRepository**: Game room management (in-memory only)

## Big Two Game Rules

- **Card Order**: 3 < 4 < 5 < 6 < 7 < 8 < 9 < 10 < J < Q < K < A < 2 (2 is highest)
- **Suit Order**: Diamonds < Clubs < Hearts < Spades
- **Format**: "3D", "KH", "AS" (rank + suit)
- **First Move**: Must include 3 of Diamonds

## API Endpoints

### REST (session-based auth via X-Session-ID header)
- `POST /session` - Create session with auto-generated username
- `GET /session/validate` - Validate session (authenticated)
- `POST /room` - Create room (returns pet-name ID)
- `GET /rooms` - List all rooms
- `GET /room/{id}` - Get room details
- `POST /room/{id}/join` - Join room (authenticated)
- `DELETE /room/{id}` - Delete room (host only)
- `POST /room/{id}/bot/add` - Add AI bot to room

### WebSocket
- `GET /ws/{room_id}?session_id={session_id}` - Real-time game communication
- Message types: `CHAT`, `MOVE`, `LEAVE`, `START_GAME` (client→server)
- Message types: `PLAYERS_LIST`, `MOVE_PLAYED`, `TURN_CHANGE`, `GAME_WON`, etc. (server→client)

## Testing

### Structure
```
tests/
├── websocket_workflow_tests.rs  # Full game scenario integration tests
└── utils/                       # Test utilities and helpers
    ├── setup.rs                 # AppState and test environment setup
    ├── mocks.rs                 # Mock repositories and managers
    ├── game_builders.rs         # Helper functions for game scenarios
    ├── actions.rs               # Common test actions (join room, make move)
    └── assertions.rs            # Game state assertions
```

### Test Categories
- **Unit tests**: Mock repositories, no external dependencies
- **Integration tests**: Use test database, marked with `#[ignore]`
- **Workflow tests**: Full game scenarios from start to finish

### Running Tests
```bash
cargo test                    # Unit tests only
cargo test -- --ignored      # Integration tests (requires DB)
cargo test test_name -- --nocapture  # Single test with output
```

## Event-Driven Flow

1. **WebSocket Message** → `websockets/handler.rs` processes incoming message
2. **Parse & Route** → Emit `RoomEvent` via `EventBus` to appropriate handlers
3. **Event Processing** → Multiple subscribers react to events:
   - `game/game_room_subscriber.rs` - Game logic (moves, turns, win conditions)
   - `bot/bot_room_subscriber.rs` - Bot responses to game events
   - `websockets/websocket_room_subscriber.rs` - WebSocket broadcasts
4. **WebSocket Response** → Events trigger broadcasts to all players in room

This separation allows game logic to be independent of WebSocket implementation and enables easy addition of bots and other event subscribers.

## Bot System

The backend includes AI bots for testing and single-player gameplay:
- **Bot creation**: Add bots via REST endpoint `POST /room/{id}/bot/add`
- **Bot strategy**: Basic strategy that plays valid moves automatically
- **Event-driven**: Bots respond to `TurnChange` events with automatic moves
- **Integration**: Bots appear as regular players to other clients