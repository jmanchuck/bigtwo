## Stats Implementation Plan

### Step 0 – Prep & Alignment
- Confirm available game-event payloads satisfy the `GameResult` structure; note any gaps for future account-based persistence.
- Define module layout under `src/stats/` and sketch public API surface that keeps future Postgres persistence swap-friendly.
- Decide on configuration or feature flags (e.g., `stats_persistence=memory|postgres`) needed later.

**Step 0 Findings (2025-10-24)**
- `RoomEvent::GameWon` gives winner UUID and is emitted while the winning `Game` remains in `GameRepository`; a stats subscriber can fetch full state via `GameService::get_game` before `GameReset` runs.
- `Game` exposes `players()` (UUID + cards) and `starting_hands()` (keyed by player UUID), enabling cards-remaining capture and future hand analysis.
- Sequential game number can derive from per-room stats state (`games_played + 1`), satisfying the in-memory counter requirement.
- `BotManager::get_bots_in_room` already supports a `had_bots` flag for completed games.
- No completion timestamp is stored; we will stamp `Utc::now()` when persisting the `GameResult`.

### Step 1 – Domain & Repository Foundations
- Add core data structs (`GameResult`, `PlayerGameResult`, `RoomStats`, `PlayerStats`) plus `StatsError`.
- Define `StatCollector`, `ScoreCalculator`, and `StatsRepository` traits with documentation focused on long-term pluggability.
- Implement `InMemoryStatsRepository` with concurrency-safe interior mutability; cover CRUD behavior and concurrency cases in unit tests.

**Step 1 Planning Notes (2025-10-24)**
- Create `src/stats/mod.rs` exporting `collectors`, `calculators`, `models`, `repository`, and `service` submodules.
- Initial layout:
  - `collectors/mod.rs`, plus `cards_remaining.rs`, `win_loss.rs`.
  - `calculators/mod.rs`, plus `card_count.rs`, `ten_plus_multiplier.rs`.
  - `models.rs` for data structs and supporting enums.
  - `repository.rs` for `StatsRepository`, `InMemoryStatsRepository`.
  - `service.rs` for `StatsService` builder/orchestrator.
  - `errors.rs` for `StatsError` (re-exported via `mod.rs`).
- Keep module public interface small via re-exports in `mod.rs`, e.g. `pub use models::*; pub use service::StatsService;`.
- Plan to share `CollectedData` enum between collectors/calculators; live alongside models to avoid cyclic deps.

### Step 2 – Collector Implementations
- Build `CardsRemainingCollector` and `WinLossCollector`, each returning strongly typed `CollectedData` payloads.
- Add focused tests using lightweight `Game` fixtures; include bot participation and empty-room edge cases to validate extensibility.

**Configuration Considerations**
- Expose stats via `AppState` so services/events can access `StatsService`; add builder methods for stats repo/service.
- Allow selecting stats persistence via env var (e.g., `STATS_BACKEND=postgres|memory`), mirroring session config; default to in-memory.
- Defer Postgres implementation; plan trait and builder hooks now so swap is frictionless later.

### Step 3 – Calculator Pipeline
- Implement `