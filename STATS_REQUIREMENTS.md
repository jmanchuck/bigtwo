# Stats Tracking System - Requirements

## 1. Overview

### Goals
- Track game statistics across multiple games within a room
- Display player performance metrics in real-time
- Provide a modular, extensible system for capturing and calculating stats
- Support future migration from session-based to account-based persistence

### Scope
- **In Scope**: Win/loss tracking, cards remaining counting, score calculation with multipliers, room-level statistics display
- **Out of Scope (v1)**: Global leaderboards, achievements, historical graphs, permanent account storage
- **Future**: User accounts, cross-room statistics, advanced analytics

## 2. Requirements

### 2.1 Data Collection Requirements

#### What to Capture Per Game
- **Game Metadata**
  - Room ID
  - Game number (sequential counter within room)
  - Completion timestamp
  - Winner UUID
  - Bot participation flag (for future filtering)
  - Game number increments per room in memory for the room's lifetime (resets when the room is torn down)

#### What to Capture Per Player
- **Cards Remaining**: Number of cards in hand when game ends (0 for winner)
- **Win/Loss**: Boolean outcome per player
- **Starting Hand**: Already captured in `Game.starting_hands` - reference for potential future analysis

### 2.2 Calculation Requirements

#### Scoring Rules (Initial Implementation)
1. **Base Score**: Cards remaining count (linear)
   - Winner: 0 points
   - Losers: 1 point per card remaining

2. **10+ Cards Multiplier**:
   - If player has ≥10 cards remaining: score × 2
   - Example: 12 cards → 12 × 2 = 24 points

3. **Score Interpretation**:
   - Scores are always ≥ 0; lower totals indicate better performance
   - Winners remain at 0 for that game; losers accumulate positive points
   - Compare players by smallest non-negative cumulative score

#### Aggregate Statistics (Per Player, Per Room)
- **Games Played**: Total games completed in this room
- **Wins**: Number of games won
- **Total Score**: Cumulative score across all games (sum of per-game scores)
- **Current Win Streak**: Consecutive wins (resets on loss)
- **Best Win Streak**: Highest streak achieved in this room

### 2.3 Bot Game Handling
- Track bot games the same as human-only games
- Set `had_bots` flag on `GameResult` for potential future filtering
- Bot stats displayed alongside human stats

### 2.4 Storage Requirements

#### Session-Based (v1)
- Stats stored in-memory per room
- Stats persist for lifetime of room (until all players leave)
- Stats lost on server restart
- No database persistence initially

#### Future Account-Based
- Migrate to PostgreSQL when user accounts are added
- Stats table will reference user accounts instead of session UUIDs
- Architecture should support this migration without major refactoring

## 3. Interface Design (Modularity)

### 3.1 Pluggable Collectors
Allow adding new types of data to capture without modifying core logic.

#### Collector Interface
```rust
#[async_trait]
pub trait StatCollector: Send + Sync {
    async fn collect(
        &self,
        game: &Game,
        winner_uuid: &str,
    ) -> Result<CollectedData, StatsError>;
}
```

#### Initial Collectors
- **CardsRemainingCollector**: Reads `game.players()` and counts `.cards.len()`
- **WinLossCollector**: Records winner UUID

#### Future Collectors (Examples)
- **TurnDurationCollector**: Track average time per turn
- **PassCountCollector**: Count how many times each player passed
- **HandTypeCollector**: Track frequency of singles/pairs/straights played

### 3.2 Pluggable Calculators
Allow adding new scoring rules and multipliers without modifying core logic.

#### Calculator Interface
```rust
pub trait ScoreCalculator: Send + Sync {
    fn calculate(
        &self,
        player_uuid: &str,
        collected_data: &[CollectedData],
        context: &CalculationContext,
    ) -> i32;

    fn priority(&self) -> u32;  // Execution order (higher = later)
}
```

#### Initial Calculators
- **CardCountScoreCalculator** (priority 100): Base score = cards remaining
- **TenPlusMultiplierCalculator** (priority 200): Apply 2× multiplier if ≥10 cards

#### Future Calculators (Examples)
- **TwosRemainingBonusCalculator**: Extra penalty for 2s (highest cards) remaining
- **FirstMoveAdvantageCalculator**: Bonus for player who went first
- **PassPenaltyCalculator**: Penalty points for excessive passing

### 3.3 Orchestration Service

**StatsService** coordinates collectors and calculators:
1. Run all collectors to gather data
2. Run calculators in priority order to compute scores
3. Update repository with results
4. Broadcast updates via WebSocket

## 4. Data Models

### 4.1 Core Structures

```rust
pub struct GameResult {
    pub room_id: String,
    pub game_number: u32,
    pub winner_uuid: String,
    pub players: Vec<PlayerGameResult>,
    pub completed_at: DateTime<Utc>,
    pub had_bots: bool,
}

pub struct PlayerGameResult {
    pub uuid: String,
    pub cards_remaining: u8,
    pub raw_score: i32,         // Before multipliers
    pub final_score: i32,       // After multipliers
}

pub struct RoomStats {
    pub room_id: String,
    pub games_played: u32,
    pub player_stats: HashMap<String, PlayerStats>,
}

pub struct PlayerStats {
    pub uuid: String,
    pub games_played: u32,
    pub wins: u32,
    pub total_score: i32,
    pub current_win_streak: u32,
    pub best_win_streak: u32,
}
```

**Name Resolution**: Player UUIDs remain the canonical identifiers (matching existing room/game logic). `StatsService` resolves a stable display name for UI consumption via the player-mapping subsystem; when no explicit name exists, fall back to the generated player name assigned when the user joined the room.

### 4.2 Repository Pattern

```rust
#[async_trait]
pub trait StatsRepository: Send + Sync {
    async fn record_game(
        &self,
        room_id: &str,
        game_result: GameResult,
    ) -> Result<(), StatsError>;

    async fn get_room_stats(
        &self,
        room_id: &str,
    ) -> Result<Option<RoomStats>, StatsError>;

    async fn reset_room_stats(
        &self,
        room_id: &str,
    ) -> Result<(), StatsError>;
}
```

**Implementations:**
- `InMemoryStatsRepository`: HashMap-based, ephemeral
- `PostgresStatsRepository`: (future) Database-backed, persistent

## 5. UI Design

### 5.1 Display Location
**GameRoom.tsx** - Stats panel shown below player list in lobby view

### 5.2 Stats Panel Components

#### Room-Level Stats
```
┌─────────────────────────┐
│   Room Statistics       │
├─────────────────────────┤
│   Games Played: 5       │
└─────────────────────────┘
```

#### Per-Player Stats
```
┌─────────────────────────────────────┐
│ Player1                             │
│ 3 wins • -12 pts • 🔥 2 win streak │
├─────────────────────────────────────┤
│ Player2                             │
│ 2 wins • +15 pts                    │
├─────────────────────────────────────┤
│ BotEasy                             │
│ 0 wins • +28 pts                    │
└─────────────────────────────────────┘
```

### 5.3 Display Rules
- **Wins**: Show total wins in this room
- **Score**: Show cumulative total score with sign (- = good, + = bad)
- **Win Streak**: Only show if ≥ 2 consecutive wins (with 🔥 emoji)
- **Sort Order**: By wins descending, then by score ascending (best first)
- **Bot Indication**: Show bot name with special styling/badge

### 5.4 Update Behavior
- **Initial Load**: Fetch stats via REST API on room join
- **Real-Time**: Subscribe to `STATS_UPDATE` WebSocket messages after each game
- **Animation**: Brief highlight/flash when stats update

## 6. Storage Strategy

### 6.1 In-Memory Storage (v1)

#### Data Structure
```rust
HashMap<String, RoomStats>  // room_id -> stats
```

#### Lifecycle
- Stats created when first game completes in room
- Stats persist while room exists
- Stats deleted when room is deleted (all players leave)
- Stats lost on server restart

#### Pros
- Fast, no database setup needed
- Simple implementation
- Works with `--memory` mode

#### Cons
- Ephemeral (lost on restart)
- Not tied to user accounts
- Limited to room scope

### 6.2 PostgreSQL Storage (Future)

#### Schema Design (Preliminary)
```sql
CREATE TABLE game_results (
    id SERIAL PRIMARY KEY,
    room_id VARCHAR(255) NOT NULL,
    game_number INT NOT NULL,
    winner_uuid UUID NOT NULL,
    completed_at TIMESTAMP NOT NULL,
    had_bots BOOLEAN DEFAULT FALSE
);

CREATE TABLE player_game_results (
    id SERIAL PRIMARY KEY,
    game_result_id INT REFERENCES game_results(id),
    player_uuid UUID NOT NULL,
    cards_remaining SMALLINT NOT NULL,
    raw_score INT NOT NULL,
    final_score INT NOT NULL
);

CREATE TABLE player_room_stats (
    room_id VARCHAR(255) NOT NULL,
    player_uuid UUID NOT NULL,
    games_played INT DEFAULT 0,
    wins INT DEFAULT 0,
    total_score INT DEFAULT 0,
    current_win_streak INT DEFAULT 0,
    best_win_streak INT DEFAULT 0,
    PRIMARY KEY (room_id, player_uuid)
);
```

#### Migration Strategy
- Keep same `StatsRepository` trait interface
- Add `PostgresStatsRepository` implementation
- Use smart configuration (like `SessionRepository`) to choose implementation
- No changes to `StatsService` or event handlers

## 7. Implementation Phases

### Phase 1: Core Infrastructure
**Goal**: Set up module structure and base traits

- [ ] Create `src/stats/` module structure
- [ ] Define `StatCollector` and `ScoreCalculator` traits
- [ ] Define data models (`GameResult`, `RoomStats`, `PlayerStats`)
- [ ] Define `StatsRepository` trait
- [ ] Implement `InMemoryStatsRepository`

### Phase 2: Collectors & Calculators
**Goal**: Implement initial data collection and scoring logic

- [ ] Implement `CardsRemainingCollector`
- [ ] Implement `WinLossCollector`
- [ ] Implement `CardCountScoreCalculator`
- [ ] Implement `TenPlusMultiplierCalculator`
- [ ] Write unit tests for collectors and calculators

### Phase 3: Service Layer
**Goal**: Orchestrate stats capture and storage

- [ ] Implement `StatsService` with builder pattern
- [ ] Add collector orchestration logic
- [ ] Add calculator orchestration (with priority sorting)
- [ ] Add repository integration
- [ ] Write unit tests with mocked repository

### Phase 4: Event Integration
**Goal**: Hook into game completion events

- [ ] Implement `StatsRoomSubscriber`
- [ ] Listen to `GameWon` event
- [ ] Call `StatsService.record_game_completion()`
- [ ] Register subscriber in `main.rs`
- [ ] Write integration test (full game → verify stats recorded)

### Phase 5: API Layer
**Goal**: Expose stats via REST and WebSocket

- [ ] Add `GET /room/{room_id}/stats` endpoint
- [ ] Add `POST /room/{room_id}/stats/reset` endpoint (host only)
- [ ] Add `STATS_UPDATE` WebSocket message type
- [ ] Broadcast stats after each game completion
- [ ] Update OpenAPI spec

### Phase 6: Frontend Integration
**Goal**: Display stats in UI

- [ ] Generate TypeScript types from updated OpenAPI spec
- [ ] Create `StatsPanel` component
- [ ] Integrate into `GameRoom.tsx`
- [ ] Fetch initial stats on room join
- [ ] Subscribe to `STATS_UPDATE` WebSocket messages
- [ ] Add stats formatting and display logic
- [ ] Add Tailwind styling

### Phase 7: Testing & Polish
**Goal**: Ensure reliability and good UX

- [ ] Write workflow test: multiple games in succession
- [ ] Test score calculations with edge cases (0 cards, 13 cards, etc.)
- [ ] Test win streak logic
- [ ] Test bot game tracking
- [ ] Manual testing with real games
- [ ] UI polish (animations, responsive design)

## 8. Open Questions

### Technical Decisions
- [ ] Should stats reset when room host changes?
- [ ] Should we expose stats via REST API or only via WebSocket?
- [ ] How to handle players rejoining room (same UUID)?
- [ ] Should stats be visible during game or only in lobby?

### UX Decisions
- [ ] Should there be a "Reset Stats" button visible to players?
- [ ] How prominent should bot stats be vs human stats?
- [ ] Should we show per-game history or only aggregates?
- [ ] Should negative scores be shown as negative or as "deficit"?

### Future Features
- [ ] Achievement system (e.g., "Win 3 in a row")
- [ ] Historical game log (list of past games)
- [ ] Export stats as JSON/CSV
- [ ] Per-player stat pages (when accounts exist)
- [ ] Elo/ranking system

## 9. Success Metrics

### Functional Requirements Met
- ✓ Games played counter increments after each game
- ✓ Win counts accurately track winners
- ✓ Scores calculated correctly with 10+ multiplier
- ✓ Win streaks tracked and reset properly
- ✓ Stats persist for room lifetime
- ✓ Stats visible in UI in real-time

### Code Quality
- ✓ Modular design allows easy addition of new collectors/calculators
- ✓ All traits properly documented
- ✓ Unit test coverage >80%
- ✓ Integration tests pass
- ✓ No performance degradation (stats calculation <10ms)

### User Experience
- ✓ Stats display clearly and understandably
- ✓ Real-time updates feel responsive
- ✓ Stats enhance competitive gameplay
- ✓ No confusion about score interpretation

## 10. Future Enhancements

### Short-Term (Next 3-6 months)
- Add more calculators (2s remaining bonus, pass penalty)
- Add per-game history view
- Add stats reset functionality
- Add room-level leaderboard

### Medium-Term (6-12 months)
- Migrate to PostgreSQL storage
- Add user account system
- Cross-room statistics
- Historical graphs and trends

### Long-Term (1+ years)
- Global leaderboards
- Achievement system
- Elo/MMR ranking
- Tournament mode with bracket tracking
- Advanced analytics (move quality, decision time, etc.)
