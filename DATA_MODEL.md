# Big Two Game Data Model

## Overview

This document outlines the proposed data model for the Big Two game system, designed to support:
- Temporary lobbies for player matchmaking
- Fast in-memory gameplay
- Persistent game history and records
- Multi-game tournament/session tracking
- Player statistics and analytics

## Current vs. Proposed Architecture

### Current Issues
- **Conceptual Overlap**: Both `Room` and `Game` track players differently
- **Lifecycle Confusion**: Rooms persist during active games unnecessarily
- **No Persistence**: Games disappear when complete, no history
- **Missing Tournament Support**: No way to track multi-game sessions

### Proposed Solution
```
Lobby (temporary) -> Game (in-memory) -> GameRecord (persistent) -> Session (tournament)
```

## Core Entities

### 1. GameSession (Tournament/Series)
**Purpose**: Track multi-game sessions between the same group of players
**Lifecycle**: Created when first game starts, persists across multiple games
**Storage**: Always persistent (PostgreSQL)

```rust
pub struct GameSession {
    pub id: String,                              // UUID
    pub players: Vec<String>,                    // Player names in this session
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub status: SessionStatus,
    pub aggregate_scores: HashMap<String, i32>,  // Total cards remaining across all games
    pub games_played: i32,
    pub current_game_id: Option<String>,         // Link to active game
}

pub enum SessionStatus {
    Active,      // Currently playing or between games
    Completed,   // Players finished the session
    Abandoned,   // Players left without completing
}
```

### 2. GameRecord (Completed Game History)
**Purpose**: Persistent record of completed games with full history
**Lifecycle**: Created when game completes, never deleted
**Storage**: Always persistent (PostgreSQL)

```rust
pub struct GameRecord {
    pub id: String,                              // Game ID (matches the Game.id during play)
    pub session_id: String,                      // Link to GameSession
    pub players: Vec<String>,                    // Player names (order matters)
    pub moves: Vec<MoveRecord>,                  // Complete move history
    pub initial_hands: HashMap<String, Vec<Card>>, // Starting cards for each player
    pub final_hands: HashMap<String, Vec<Card>>,   // Ending cards for each player
    pub winner: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration_seconds: i32,
}

pub struct MoveRecord {
    pub player: String,
    pub cards: Vec<Card>,                        // Empty vec for pass
    pub timestamp: DateTime<Utc>,
    pub turn_number: i32,
}
```

### 3. Game (Active Game State)
**Purpose**: In-memory state for currently active games
**Lifecycle**: Created when game starts, deleted when game ends (after persisting to GameRecord)
**Storage**: Memory only (GameManager)

```rust
// Current Game struct stays mostly the same, but enhanced:
pub struct Game {
    pub id: String,                              // Matches future GameRecord.id
    pub session_id: String,                      // Link to GameSession
    pub players: Vec<Player>,
    pub current_turn: usize,
    pub consecutive_passes: usize,
    pub last_played_cards: Vec<Card>,
    pub move_history: Vec<MoveRecord>,           // Track moves for persistence
    pub started_at: DateTime<Utc>,
}
```

### 4. Lobby (Temporary Waiting Area)
**Purpose**: Pre-game waiting area for players to gather
**Lifecycle**: Created when host creates room, deleted when game starts
**Storage**: Memory only (current Room repository, renamed)

```rust
// Rename current RoomModel to LobbyModel
pub struct Lobby {
    pub id: String,                              // Pet name ID
    pub host: String,
    pub players: Vec<String>,
    pub status: LobbyStatus,
    pub created_at: DateTime<Utc>,
    pub session_id: Option<String>,              // Link to existing session if rejoining
}

pub enum LobbyStatus {
    Waiting,     // Waiting for players
    Starting,    // Game is being created
    InGame,      // Game in progress (lobby should be deleted soon)
}
```

## Database Schema (PostgreSQL)

```sql
-- Game Sessions (tournaments)
CREATE TABLE game_sessions (
    id VARCHAR PRIMARY KEY,
    players JSONB NOT NULL,                     -- Array of player names
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_activity TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    status VARCHAR NOT NULL,                    -- 'active', 'completed', 'abandoned'
    aggregate_scores JSONB NOT NULL,            -- Map of player -> total cards remaining
    games_played INTEGER DEFAULT 0,
    current_game_id VARCHAR                     -- NULL if between games
);

-- Completed Game Records
CREATE TABLE game_records (
    id VARCHAR PRIMARY KEY,
    session_id VARCHAR NOT NULL REFERENCES game_sessions(id),
    players JSONB NOT NULL,                     -- Array of player names in turn order
    moves JSONB NOT NULL,                       -- Array of move records
    initial_hands JSONB NOT NULL,               -- Map of player -> starting cards
    final_hands JSONB NOT NULL,                 -- Map of player -> ending cards
    winner VARCHAR NOT NULL,
    started_at TIMESTAMP WITH TIME ZONE NOT NULL,
    completed_at TIMESTAMP WITH TIME ZONE NOT NULL,
    duration_seconds INTEGER NOT NULL
);

-- Indexes for performance
CREATE INDEX idx_game_sessions_status ON game_sessions(status);
CREATE INDEX idx_game_sessions_last_activity ON game_sessions(last_activity);
CREATE INDEX idx_game_records_session_id ON game_records(session_id);
CREATE INDEX idx_game_records_completed_at ON game_records(completed_at);
```

## Lifecycle Flow

### 1. Starting a New Session
```
1. Players gather in Lobby (memory)
2. Host starts game
3. GameSession created (DB)
4. Game created (memory) with session_id
5. Lobby deleted (memory)
```

### 2. During Gameplay
```
1. All game state in memory for performance
2. Moves tracked in Game.move_history
3. No database writes during active play
```

### 3. Completing a Game
```
1. Game ends, winner determined
2. GameRecord created and saved to DB
3. GameSession.aggregate_scores updated
4. Game deleted from memory
5. Players can start new game in same session
```

### 4. Session Management
```
1. Sessions persist across multiple games
2. Players can rejoin existing sessions
3. Background cleanup for abandoned sessions
4. Session statistics calculated from GameRecords
```

## Repository Pattern

### Current Repositories (to modify)
- `RoomRepository` -> `LobbyRepository` (memory only)
- Add `GameSessionRepository` (DB persistent)
- Add `GameRecordRepository` (DB persistent)

### New Repository Interfaces
```rust
#[async_trait]
pub trait GameSessionRepository {
    async fn create_session(&self, session: &GameSession) -> Result<(), AppError>;
    async fn get_session(&self, session_id: &str) -> Result<Option<GameSession>, AppError>;
    async fn update_session(&self, session: &GameSession) -> Result<(), AppError>;
    async fn find_active_sessions_for_players(&self, players: &[String]) -> Result<Vec<GameSession>, AppError>;
    async fn cleanup_abandoned_sessions(&self, cutoff: DateTime<Utc>) -> Result<u64, AppError>;
}

#[async_trait]
pub trait GameRecordRepository {
    async fn save_game_record(&self, record: &GameRecord) -> Result<(), AppError>;
    async fn get_game_record(&self, game_id: &str) -> Result<Option<GameRecord>, AppError>;
    async fn get_session_games(&self, session_id: &str) -> Result<Vec<GameRecord>, AppError>;
    async fn get_player_game_history(&self, player: &str, limit: Option<i32>) -> Result<Vec<GameRecord>, AppError>;
}
```

## Benefits of This Design

1. **Performance**: Active games stay in memory, no DB writes during play
2. **Persistence**: Complete game history preserved forever
3. **Tournament Support**: Multi-game sessions tracked properly
4. **Clean Lifecycle**: Clear separation between temporary lobbies and persistent records
5. **Analytics Ready**: Rich data for player statistics and game analysis
6. **Scalable**: Can add features like rankings, tournaments, etc.

## Migration Strategy

1. **Phase 1**: Rename Room -> Lobby, keep current behavior
2. **Phase 2**: Add GameSession and GameRecord entities
3. **Phase 3**: Modify game completion to persist records
4. **Phase 4**: Add session management and multi-game support
5. **Phase 5**: Add analytics and advanced features

## Future Enhancements

- Player rankings and ELO ratings
- Tournament brackets
- Game replay functionality
- Advanced statistics and analytics
- Spectator mode
- Chat message history