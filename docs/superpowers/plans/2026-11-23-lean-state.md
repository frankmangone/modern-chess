# Lean state Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (native). Steps use checkbox syntax.

**Goal:** Cheap names and no piece copies in the hot path, with identical moves and faster timing.

**Architecture:** A global interner gives `Position` and a new `Symbol` (codes, players, flags) `Copy` ids that compare, order, print and save as their text. A candidate move records `Removal` snapshots. Maps keyed by ids use a fast hasher. Attack sets are cached per position.

**Tech Stack:** Rust (crate `modern-chess-engine`, lib `modern_chess`; also `crates/game`, `crates/session`).

**Spec:** `docs/superpowers/specs/2026-11-23-lean-state-design.md`

## Global Constraints

- Move counts identical (`catalogue_counts`, speed example `moves=`); legal moves unchanged.
- Saved states load and save unchanged (JSON shape, key order, `position_map` keys).
- Release timing run beside `main` (`6ccb435`, alternating) after each task: chess and Shogi faster, Go not slower. Baseline on this machine: chess 87.1 ms, Shogi 250.2 ms, Go 10.1 ms.
- Commits: conventional, end with `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. No push.

## Review Focus

- An id must order and compare as its text (`"2,0"` before `"10,0"`, names after coordinates).
- The interner must be safe across threads (the game and session crates run tests in parallel).
- `Borrow<str>` lookups of `Position` keys go away: every `HashMap<Position, _>` lookup by `&str` must still work.
- Saved games with positions as arrays, strings and the empty name `[]` round-trip.
- The attack-set cache must be dropped whenever the pieces change (play, undo, restore).

### Task 1: Interner, `Symbol`, `Position` as an id

**Files:** `crates/engine/src/shared/{intern.rs (new),position.rs,mod.rs}`, compile fixes across the engine, game and session crates, tests in `shared/` and `tests/`.

**Interfaces:** Produces `shared::Symbol` (`Copy`; `new(&str)`, `as_str() -> &'static str`, `Deref<Target=str>`, `From<&str>/From<String>`, `PartialEq<str>/<&str>/<String>`, `Display`, serde as string, `Hash` by id, `Ord` by text) and `Position(u32)` with today's API.

- [ ] Tests first: `symbol_equals_its_text`, `symbol_orders_by_text`, `same_text_same_id`, `position_orders_coordinates_before_names`, `position_serialises_as_before` (array, string, `[]`), `interner_is_shared_between_threads`.
- [ ] Implement the interner (`RwLock<Interner>`, leaked `&'static str`, `HashMap<&'static str, u32>`, `Vec<&'static str>`), `Symbol`, `Position(u32)`; replace `Borrow<str>` uses.
- [ ] `cargo test --workspace` green; speed run; commit.

### Task 2: Pieces use `Symbol`

**Files:** `logic/game/state/piece.rs` and every use of `Piece.code`, `.player`, `.state` keys (`HashMap<Symbol, PieceState>`), turn order, `hand`/`captured`/`counters` keys stay strings at the edges unless a profile says otherwise.

- [ ] Tests: a piece with flags saves and loads as before; `tick_state_flags` unchanged.
- [ ] `Piece { code: Symbol, player: Symbol, total_moves, state: HashMap<Symbol, PieceState> }`; fix compile errors; suite green; speed; commit.

### Task 3: `Removal` snapshots

**Files:** `logic/game/ways/{mod,run}.rs`, `blueprint/test_block/judge.rs`, `handlers/choose.rs`, tests.

**Interfaces:** `Way.removed: Vec<Removal>`, `Removal { code: Symbol, player: Symbol, moved: bool }`, `TestContext.removed: &[Removal]`.

- [ ] Test: `REMOVED` sees code, owner and has-moved of a removed piece (including a piece moved earlier in the same way).
- [ ] Implement; remove other `Piece` clones in the runner that the profile points at; suite green; speed; commit.

### Task 4: One fast hasher

**Files:** `shared/` (`IdMap`, `IdSet`, `IdHasher`), maps keyed by `Position` or `Symbol` across the engine.

- [ ] Test: iteration-order-free outputs unchanged (suite). Implement; speed; commit. Drop if the profile shows no gain.

### Task 5: Attack sets once per position

**Files:** `logic/game/attack.rs`, `logic/game/mod.rs`, callers of `calculate_threats_with`.

- [ ] Test: `cached_threats_equal_direct_threats` over every catalogue game, two plies. Cache built on first ask, dropped on any change to the pieces. Suite; speed; commit.

### Task 6: Profile again, remaining clones; docs; final

- [ ] Re-profile; remove the next clone sites that cost over 2%; known-limitations and route doc updated (slice 5 built; attack-set legality findings stay carried); final speed table in the ledger; whole-branch review on opus and one fix pass.
