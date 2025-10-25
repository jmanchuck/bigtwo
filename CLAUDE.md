# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is the Rust backend for Big Two, a real-time multiplayer card game. Built with axum + tokio, it features event-driven architecture, WebSocket communication, and complete Big Two gameplay mechanics.

## Development Commands

```bash
# Development (auto-detects PostgreSQL or falls back to in-memory)
./scripts/dev.sh --postgres    # Force PostgreSQL
./scripts/dev.sh --memory      # Force in-memory storage

# Build and test (IMPORTANT: Always run these before committing)
cargo check                    # Fast compile check
cargo test                     # Run unit tests
cargo test -- --ignored        # Run integration tests (requires DB)
cargo test test_name -- --nocapture  # Run single test with output
cargo clippy                   # Lint (must pass with no warnings)
cargo clippy -- -D warnings    # Treat warnings as errors
cargo fmt                      # Format code
cargo fmt -- --check           # Check formatting without modifying

# Database (when using PostgreSQL)
sqlx migrate run              # Apply migrations
sqlx migrate add <name>       # Create migration

# Manual testing
./scripts/test-session.sh     # Test REST endpoints
```

**IMPORTANT**: Before submitting any code changes:
1. Run `cargo test` - All tests must pass
2. Run `cargo clippy` - All clippy warnings must be fixed
3. Run `cargo fmt` - Code must be properly formatted

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
├── stats/                    # Game statistics tracking system
│   ├── models.rs            # Data structures (GameResult, RoomStats, PlayerStats)
│   ├── service.rs           # Stats service and room subscriber
│   ├── repository.rs        # Stats storage (in-memory with per-room locking)
│   ├── collectors/          # Data collectors (cards remaining, win/loss)
│   └── calculators/         # Score calculators (card count, 10+ multiplier)
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

### StatsService (stats/service.rs)
Tracks game statistics per room. Uses collector pattern for data gathering and calculator pattern for score computation. Automatic reset when room empties.

### Repository Pattern
- **SessionRepository**: JWT session storage (in-memory or PostgreSQL)
- **RoomRepository**: Game room management (in-memory only)
- **StatsRepository**: Per-room statistics (in-memory with per-room locking)

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
- `GET /room/{id}/stats` - Get current stats for room (games played, player stats)
- `POST /room/{id}/join` - Join room (authenticated)
- `DELETE /room/{id}` - Delete room (host only)
- `POST /room/{id}/bot/add` - Add AI bot to room
- `DELETE /room/{id}/bot/{bot_uuid}` - Remove bot from room

### WebSocket
- `GET /ws/{room_id}` - Real-time game communication (JWT auth via `Sec-WebSocket-Protocol` header)
- Message types: `CHAT`, `MOVE`, `LEAVE`, `START_GAME`, `READY` (client→server)
- Message types: `PLAYERS_LIST`, `MOVE_PLAYED`, `TURN_CHANGE`, `GAME_STARTED`, `GAME_WON`, `GAME_RESET`, `BOT_ADDED`, `BOT_REMOVED`, `STATS_UPDATED`, `ERROR`, `HOST_CHANGE` (server→client)

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
   - `stats/service.rs` (StatsRoomSubscriber) - Statistics tracking
   - `websockets/websocket_room_subscriber.rs` - WebSocket broadcasts
4. **WebSocket Response** → Events trigger broadcasts to all players in room

This separation allows game logic to be independent of WebSocket implementation and enables easy addition of bots and other event subscribers.

## Code Quality Requirements

**CRITICAL**: Always run these commands before committing:
1. **`cargo test`** - All unit tests must pass
2. **`cargo clippy`** - Zero warnings allowed (CI enforces `-D warnings`)
3. **`cargo fmt`** - Code must be formatted

**Code Style Guidelines**:
- **Idiomatic Rust**: Use `Result`, `Option`, `?`, traits, proper error handling
- **No unsafe code**: Use clippy-friendly patterns
- **Event-driven patterns**: Emit events through EventBus rather than direct function calls
- **Dependency injection**: Use traits for services to enable mocking in tests
- **Focus on the task**: Do not change unrelated code even if it could be improved

## Bot System

The backend includes AI bots for testing and single-player gameplay:
- **Bot creation**: Add bots via REST endpoint `POST /room/{id}/bot/add`
- **Bot strategy**: Basic strategy that plays valid moves automatically
- **Event-driven**: Bots respond to `TurnChange` events with automatic moves
- **Integration**: Bots appear as regular players to other clients

## Stats System

The backend tracks game statistics per room:
- **Data collectors**: `CardsRemainingCollector`, `WinLossCollector` gather game data
- **Score calculators**: `CardCountScoreCalculator`, `TenPlusMultiplierCalculator` compute scores with priorities
- **Per-room locking**: Thread-safe statistics updates using per-room mutexes
- **Automatic reset**: Stats reset when room becomes empty (no human players)
- **Bot filtering**: Bots are excluded from statistics tracking
- **REST endpoint**: `GET /room/{id}/stats` to fetch current statistics
- **WebSocket updates**: `STATS_UPDATED` message broadcast after each game