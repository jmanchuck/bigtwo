# Big Two Game Requirements

## Current Features

### Authentication & Sessions
- **Auto-login**: Users get auto-generated pet names (e.g. "happy-dog-42")
- **Session management**: 7-day JWT sessions, no registration required
- **Session persistence**: Works with PostgreSQL, in-memory storage for testing

### Room Management
- **Room creation**: Host creates room with pet-name ID
- **Room discovery**: List all active rooms
- **Room joining**: Players join rooms (max 4 players)
- **Host privileges**: Host can start games and manage room

### Real-time Gameplay
- **Complete Big Two rules**: Card ranking, suit order, turn progression
- **Move validation**: Server validates all card plays according to rules
- **Turn management**: Automatic turn progression with pass mechanics
- **Win detection**: Game ends when player runs out of cards
- **WebSocket communication**: Real-time moves, chat, game state updates

### Communication
- **In-game chat**: Players can chat during gameplay
- **Game events**: Real-time notifications for joins, moves, wins

## Connection Handling (Current State)

### What Happens on Disconnect
- **Detection**: Server detects WebSocket disconnection
- **Event emission**: `PlayerDisconnected` event sent to room
- **Connection cleanup**: WebSocket removed from connection manager
- **Game impact**: ❌ Game continues without player (broken state)

### What Happens on Leave (Explicit)
- **Player action**: Player sends `LEAVE` message
- **Room update**: Player removed from room player list
- **Event emission**: `PlayerLeft` event sent to remaining players
- **Game impact**: ❌ Active games become unplayable

### Current Gaps
- ❌ **No reconnection**: Disconnected players cannot rejoin active games
- ❌ **Game breaks**: Active games become unplayable when players leave
- ❌ **No game pause**: No mechanism to pause games during disconnects
- ❌ **No timeout handling**: No distinction between temporary/permanent disconnects

## Missing Features

### 1. Reconnection System (high priority)
- **Game rejoining**: Players can reconnect to active games
- **State restoration**: Reconnected players see current game state
- **Timeout handling**: Distinguish temporary disconnects from permanent leaves
- **Turn management**: Handle turns for disconnected players (skip/timeout)

### 2. Bot Players (low priority)
- **AI opponents**: Computer players with varying difficulty levels
- **Slot filling**: Bots can fill empty slots in rooms
- **Disconnect replacement**: Bots can replace disconnected players temporarily
- **Bot management**: Host can add/remove bots from rooms

### 3. Game Persistence & History (high priority)
- **Game records**: Save completed games with full move history
- **Multi-game sessions**: Tournament mode with multiple rounds
- **Statistics tracking**: Win/loss records, player rankings
- **Game replay**: View past games move-by-move

### 4. Spectator Mode (low priority)
- **Watch games**: Join ongoing games as observer
- **Spectator chat**: Separate chat channel for spectators
- **Game discovery**: List active games available for spectating

### 5. Enhanced Room Features (low priority)
- **Room settings**: Customize game rules, bot difficulty
- **Private rooms**: Password-protected or invite-only rooms
- **Persistent lobbies**: Rooms that survive game completion

## User Experience Flows

### Happy Path
1. **Join**: Player creates/joins room → lobby shows 4 players
2. **Play**: Host starts game → players take turns → game completes
3. **Continue**: Players stay in room → host start new game or leave

### Disconnect Scenarios

#### Temporary Disconnect (Target)
1. Player loses connection during game
2. Game pauses or continues with AI substitute
3. Player reconnects within timeout window
4. Player resumes from current game state

#### Permanent Disconnect (Target)
1. Player disconnects and doesn't return
2. After timeout, bot replaces player permanently
3. Game continues normally with bot

### Bot Integration Scenarios

#### Pre-game Bot Addition
1. Host creates room with 2 human players
2. Host adds 2 bots to reach 4 players
3. Game starts with mixed human/bot players

#### Mid-game Bot Replacement
1. Human player disconnects during game
2. Bot automatically takes over their hand
3. Game continues seamlessly

### Tournament Flow (Target)
1. Players create tournament session
2. Play multiple games with same group
3. Track aggregate scores across games
4. Declare tournament winner

## Technical Requirements

### Connection State Management
- **Connection tracking**: Monitor connection health per player
- **Reconnection windows**: Define timeout periods for rejoining
- **State synchronization**: Efficiently sync game state on reconnect

### Data Persistence
- **Game session storage**: Multi-game tournament tracking
- **Move history**: Complete audit trail of all games
- **Player statistics**: Aggregated win/loss data
- **Recovery data**: Sufficient state for reconnection

### Performance Considerations
- **Memory management**: Balance between speed and memory usage
- **Database efficiency**: Minimize writes during active play
- **Scalability**: Support multiple concurrent games/tournaments
- **WebSocket optimization**: Efficient real-time message delivery

### Security & Fairness
- **Reconnection security**: Verify player identity on rejoin
- **Game integrity**: Prevent cheating through disconnect/reconnect
- **Data privacy**: Protect player statistics and game history

## Priority Levels

### Phase 1 (Critical)
- Fix disconnect handling to not break games
- Basic reconnection within active games

### Phase 2 (Important)
- Game persistence and multi-round sessions
- Player statistics and history

### Phase 3 (Nice to Have)
- Spectator mode
- Bot players
- Game stats
- Advanced room settings
- Tournament brackets and rankings