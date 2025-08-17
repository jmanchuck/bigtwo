# Game Module Documentation

## Overview

The `game` module implements the core Big Two card game logic and state management. It provides a complete implementation of Big Two rules, including card validation, turn progression, win detection, and event handling. The module is designed to be decoupled from WebSocket/HTTP concerns through an event-driven architecture.

## Architecture

The game module follows a layered architecture with clear separation of concerns:

```
game/
├── cards.rs              # Card types and Big Two hand validation
├── core.rs               # Core game state and business rules  
├── game_room_subscriber.rs # Event handler for game logic
├── repository.rs         # Game state persistence (in-memory)
├── service.rs            # Business logic orchestration
└── mod.rs               # Public API exports
```

## Key Components

### cards.rs - Card System Implementation
**Responsibility**: Defines card types, sorting rules, and hand validation for Big Two

**Key Types**:
- `Card`: Basic card with rank and suit
- `Rank`: Big Two rank ordering (3-4-5-6-7-8-9-T-J-Q-K-A-2)
- `Suit`: Suit hierarchy (Diamonds < Clubs < Hearts < Spades)
- `Hand`: Enum for all valid hand types (Single, Pair, Triple, Five-card combinations)
- `FiveCardHand`: Straight, Flush, Full House, Four of a Kind, Straight Flush

**Strengths**:
- Comprehensive Big Two rule implementation with proper rank/suit ordering
- Strong type safety with enum-based hand types
- Extensive test coverage (1300+ lines of tests)
- Proper error handling with `HandError` type
- Clean separation of concerns (parsing, validation, comparison)

**Potential Issues**:
- Very large file (1300+ lines) - could benefit from splitting
- Complex straight validation logic with special cases for Ace positioning
- Some code duplication in hand comparison logic

### core.rs - Game State and Rules
**Responsibility**: Manages game state, turn progression, and move validation

**Key Types**:
- `Game`: Main game state with players, turn tracking, move history
- `Player`: Player data (name, UUID, cards)
- `GameError`: Comprehensive error types for invalid moves

**Strengths**:
- Clean API with `play_cards()` as main entry point
- Proper turn validation and progression
- First turn validation (must include 3♦)
- Win condition detection
- Immutable game history tracking
- Good separation of validation concerns

**Potential Issues**:
- Large file with multiple responsibilities (could split validation logic)
- Card ownership validation could be more efficient with HashSet
- Starting hands stored separately from current hands (potential sync issues)

### game_room_subscriber.rs - Event Handler
**Responsibility**: Handles game-related events and orchestrates game logic

**Key Functions**:
- Creates new games when rooms start
- Processes player moves through game service
- Emits turn changes and win events
- Handles 5-second game reset timer

**Strengths**:
- Clean event-driven architecture
- Proper async handling
- Good error propagation
- Decoupled from WebSocket concerns

**Potential Issues**:
- Timer logic could be moved to a separate service
- Hard-coded 5-second reset delay
- Potential race conditions with concurrent events

### service.rs & repository.rs - Business Logic Layer
**Responsibility**: Orchestrates game operations and manages game state persistence

**Strengths**:
- Clean service layer pattern
- Repository abstraction for testability
- Player mapping integration

**Potential Issues**:
- Currently only in-memory storage (no persistence)
- No cleanup of completed games
- Player mapping service dependency could be better abstracted

## Data Flow

1. **Game Creation**: `POST /room/{id}/start` → Event → `GameEventRoomSubscriber` → `GameService.create_game()`
2. **Move Execution**: WebSocket move → Event → `GameEventRoomSubscriber` → `GameService.try_play_move()` → `Game.play_cards()`
3. **State Updates**: Game logic updates → Events → WebSocket broadcasts

## Big Two Rules Implementation

### Card Ordering
- **Ranks**: 3 < 4 < 5 < 6 < 7 < 8 < 9 < 10 < J < Q < K < A < 2
- **Suits**: Diamonds (0) < Clubs (1) < Hearts (2) < Spades (3)
- **Comparison**: Rank first, then suit for tiebreaking

### Hand Types
- **Single**: Any single card
- **Pair**: Two cards of same rank
- **Triple**: Three cards of same rank  
- **Five-card**: Straight, Flush, Full House, Four of a Kind, Straight Flush

### Game Rules
- 4 players, 13 cards each
- Player with 3♦ goes first and must include it in first move
- Only same hand types can be compared (except 5-card hands)
- 3 consecutive passes clear the table
- First player to empty their hand wins

## Strengths

1. **Complete Implementation**: Full Big Two rules with edge cases handled
2. **Type Safety**: Strong typing prevents invalid states
3. **Test Coverage**: Comprehensive test suite with edge cases
4. **Event-Driven**: Clean separation from transport concerns
5. **Clear API**: Simple `play_cards()` interface hides complexity

## Areas for Improvement

### Code Organization
- **cards.rs too large**: Split into separate files for different hand types
- **Validation logic**: Extract to dedicated validation module
- **Code duplication**: Common patterns in hand comparison could be abstracted

### Performance
- **Card ownership checks**: Use HashSet instead of Vec.contains()
- **Hand parsing**: Cache parsed hands to avoid re-parsing
- **Memory usage**: Clean up completed games

### Maintainability
- **Magic numbers**: Hard-coded timeouts and player counts
- **Error messages**: More descriptive error messages for debugging
- **Documentation**: Add more inline documentation for complex algorithms

### Testing
- **Integration gaps**: More tests for event integration
- **Performance tests**: Test with large numbers of games
- **Error scenarios**: More comprehensive error condition testing

## Recommended Refactoring

1. **Split cards.rs**: 
   ```
   cards/
   ├── mod.rs           # Public API
   ├── basic.rs         # Card, Rank, Suit
   ├── hands/           # Hand types
   │   ├── single.rs
   │   ├── pair.rs
   │   ├── five_card.rs
   │   └── validation.rs
   ```

2. **Extract validation**:
   ```
   validation/
   ├── turn.rs          # Turn validation
   ├── cards.rs         # Card ownership validation  
   ├── rules.rs         # Game rule validation
   ```

3. **Add configuration**:
   ```rust
   pub struct GameConfig {
       pub reset_delay_seconds: u64,
       pub max_players: usize,
       pub deck_size: usize,
   }
   ```

## Critical Issues to Address

1. **Memory Leaks**: Games are never cleaned up after completion
2. **Race Conditions**: Concurrent move attempts not handled atomically
3. **Error Recovery**: No mechanism to recover from invalid game states
4. **Player Disconnection**: No handling of players leaving mid-game

## Overall Assessment

The game module provides a solid foundation for Big Two implementation with comprehensive rule coverage and good type safety. However, it suffers from monolithic file organization and lacks some production concerns like cleanup and error recovery. The event-driven architecture is well-designed but could benefit from more sophisticated event handling patterns.

**Maintainability Score**: 7/10
**Test Coverage**: 9/10  
**Performance**: 6/10
**Architecture**: 8/10