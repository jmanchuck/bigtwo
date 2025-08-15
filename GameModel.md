# Big Two Game Data Model

## Overview

Data model covering both live game state (transient, in-memory) and game records (permanent, database) for Big Two gameplay.

## Live Game State

### Core Structures
```rust
pub struct Game {
    id: String,                    // Room identifier
    players: Vec<Player>,          // All players with current hands
    current_turn: usize,           // Index of player whose turn it is
    consecutive_passes: usize,     // Track passes for game state reset
    played_hands: Vec<Hand>,       // Complete history of all hands played
}

pub struct Player {
    name: String,                  // Player identifier
    cards: Vec<Card>,             // Current hand (remaining cards)
}
```

### State Tracking
- **Starting Hands**: Implicit (52 cards dealt evenly, 13 per player)
- **Player Names**: Stored in `players` vector (in turn order)
- **Moves Played**: Stored as `Hand` enum in `played_hands` vector (complete history)
- **Table State**: Derived from `played_hands` and `consecutive_passes` counter
- **Winner**: Detected when `player.cards.is_empty()`

### Storage Strategy
- **In-Memory**: `HashMap<String, Game>` in `GameManager`
- **Rationale**: Games are short-lived, real-time performance critical
- **Trade-off**: Simple/fast vs no persistence

## Game Records

After a game is completed, the game record is persisted to the database. A game summary is computed and displayed to the players at the end of the game. Including:

- winner
- number of cards left per player

During an active game, the in memory game state stores all played hands as `Hand` enum instances, preserving complete game history while maintaining the starting hands.

### Permanent Storage Structures
```rust
pub struct GameRecord {
    pub id: String,                     // Primary key
    pub players: Vec<String>,           // Player names in turn order
    pub starting_hands: Vec<Vec<String>>, // Cards for each player (same order as players)
    pub moves: Vec<String>,             // Complete move sequence, ordered by turn order
    pub winner: String,                 // Winner (convenience, derivable but kept)
}
```

### Query Patterns
1. Games by player
   - Queries all game records for a given player
2. Matchup between two players
   - Queries all game records for a given two players
3. Individual game
   - Queries a single game record for a given game id

### Database Schema
```sql
CREATE TABLE game_records (
    id VARCHAR PRIMARY KEY,
    players JSONB NOT NULL,           -- Array of player names in turn order
    starting_hands JSONB NOT NULL,    -- Array of card arrays (same order as players)
    moves JSONB NOT NULL,             -- Array of GameMove objects
    winner VARCHAR NOT NULL
);

-- Index for common queries
CREATE INDEX idx_game_records_winner ON game_records(winner);
```

## Storage Strategy Summary

### Live Games: In-Memory
- **Why**: Real-time performance, short-lived sessions
- **Storage**: ~1KB per active game
- **Access**: O(1) lookup by room_id

### Game Records: Database
- **Why**: Historical tracking, player statistics, game replay
- **Storage**: ~1-2KB per completed game
- **Access**: Indexed queries for statistics and history

### Data Flow
1. **During Play**: Update live `Game` state in-memory, storing all hands in `played_hands` vector
2. **On Completion**: Convert `Game` → `GameRecord` and persist to database using complete hand history
3. **For History**: Query `GameRecord` for statistics and replay