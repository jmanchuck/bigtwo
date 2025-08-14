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
│   ├── handlers.rs          # REST endpoints: create, join, list
│   ├── repository.rs        # In-memory storage (uses pet-name IDs)
│   ├── service.rs           # Business logic
│   └── models.rs, types.rs  # Data structures
├── game/                     # Big Two game logic and state
│   ├── cards.rs             # Card types, Big Two sorting rules
│   ├── logic.rs             # Core game rules, turn progression
│   ├── gamemanager.rs       # Game state management per room
│   └── game_room_subscriber.rs # Event handler for game logic
├── websockets/               # Real-time WebSocket communication
│   ├── handler.rs           # WebSocket upgrade and message routing
│   ├── messages.rs          # Message type definitions
│   ├── connection_manager.rs # Per-room connection tracking
│   ├── socket.rs            # Individual WebSocket handling
│   └── websocket_room_subscriber.rs # Event handler for WebSocket broadcast
└── utils/                    # Utility functions
```

## Key Components

### AppState (shared.rs)
Central dependency injection container holding all repositories, managers, and the event bus. Contains builder pattern for testing.

### EventBus (event/)
Central message broker enabling decoupled communication. Supports both global and room-specific event subscriptions.

### GameManager (game/gamemanager.rs)
Manages Big Two game state per room. Handles game creation, move validation, turn progression, and win detection.

### ConnectionManager (websockets/connection_manager.rs)
Tracks WebSocket connections per room for message broadcasting.

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
- `POST /room/{id}/join` - Join room (authenticated)
- `GET /room/{id}` - Get room details

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

1. **WebSocket Message** → `websockets/handler.rs`
2. **Parse & Route** → Emit `RoomEvent` via `EventBus`
3. **Game Logic** → `game/game_room_subscriber.rs` processes game events
4. **WebSocket Broadcast** → `websockets/websocket_room_subscriber.rs` sends responses

This separation allows game logic to be independent of WebSocket implementation.