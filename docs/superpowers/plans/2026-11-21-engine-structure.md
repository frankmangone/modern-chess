# Engine Structure Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Parse rule codes into enums at build, share the build-time rules behind an `Arc`, and regroup `logic` by job — with no behaviour change.

**Architecture:** Three refactors of `crates/engine` and one fix, in this order: enums (touches blueprints and win/legal/cell checks), `Arc<Rules>` (touches every `self.<rule field>`), the turn order saved in `GameState`, modules (file moves and re-exports). Other crates are respelled mechanically.

**Tech Stack:** Rust (workspace: `modern-chess-engine` lib `modern_chess`, `modern-chess-session`, `modern-chess-wasm`, `modern-chess-game`, `apps/cli`).

**Spec:** `docs/superpowers/specs/2026-11-21-engine-structure-design.md`

## Global Constraints

- No behaviour change except Task 3 (the turn order is saved): test assertions are not edited; call sites may be respelled mechanically.
- `Effect`, `MoveRecord` and every spec type keep their fields and serde shape; `GameState` gains only `turn_order` (Task 3).
- Unknown codes stay ignored (`Unknown` variants), as today.
- The engine crate denies `unwrap`, `expect` and indexing outside tests; it builds for wasm.
- Workspace builds with 0 warnings; `cargo fmt` clean.
- Conventional commits ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Game crate tests need the `modern-chess-db` container running.

## Review Focus

1. A spec-declared condition whose name collides with nothing built in (e.g. `REACH_END`) must still be evaluated as a named cell condition — `Named(String)` must catch every non-built-in code, exactly as `Unknown` did.
2. Tests or callers that mutate a rule field on a cloned game must not change the original: `Arc::make_mut` copies when shared.
3. A saved state with no `turn_order` (every state saved before this branch) loads with the spec's order, not an empty one — an empty order would leave nobody to move.
4. The session's zone list (it reads declared conditions and compares their type with `POSITION`) still lists the same zones.
5. Nothing outside the engine loses a path it imports (`Game`, `GamePhase`, `GameTransition`, `Position`, `POSITION`, `parse_game_spec(_str)`, `GameSpec`, `GameSpecError`, and the types reachable from `Game`'s public fields).

---

### Task 1: Rule codes become enums at build

**Files:**
- Modify: `crates/engine/src/logic/blueprint/move_blueprint.rs` (`Condition.code` → `kind`, `SideEffectBlueprint.action` → `kind`, build in `from_spec`)
- Modify: `crates/engine/src/logic/blueprint/conditions/mod.rs`, `conditions/*.rs` that read `code`
- Modify: `crates/engine/src/logic/blueprint/side_effects/mod.rs`
- Modify: `crates/engine/src/logic/game/win.rs`, `scoring.rs`, `legal.rs`, `conditions.rs`, `structs/condition.rs`, `main.rs`
- Modify: `crates/session/src/session.rs` (declared condition type)
- Test: `crates/engine/src/tests/rule_codes/mod.rs` (new; add to `tests/mod.rs` in sorted order)

**Interfaces:**
- Produces:
  - `pub enum ConditionKind { DependsOn, CheckState, PieceFirstMove, NotAttacked, PathNotAttacked, PathPieceCount, PieceAt, PieceNotAt, AllyAdjacentCount, LegEmpty, Named(String) }` with `impl From<&str>`; `Condition { kind: ConditionKind, .. }`.
  - `pub enum SideEffectKind { SetState, Capture, MovePiece, Convert, CopySource, Explode, Unknown }`; `SideEffectBlueprint { kind: SideEffectKind, .. }`.
  - `pub enum WinConditionKind { PieceInZone, OpponentBare, CheckCount, AreaScore, Unknown }`; `pub struct WinCondition { pub kind: WinConditionKind, pub spec: WinConditionSpec }`; `Game.win_conditions: Vec<WinCondition>`.
  - `pub enum CellConditionKind { Position, State, Unknown }`; `ConditionDef { kind: CellConditionKind, check }`.
  - `DropRestrictionCode` parsed where drop restrictions are built (keep `AllyOnFile`, `Custom`).

- [ ] **Step 1: Write the failing tests** in `tests/rule_codes/mod.rs`, building chess from `specs/chess.json`:
  - `pawn_promotion_condition_is_named`: some pawn move blueprint has a modifier condition with `kind == ConditionKind::Named("REACH_END".into())`.
  - `castling_path_condition_is_built_in`: the king's blueprints contain a condition with `kind == ConditionKind::PathPieceCount`.
  - `capture_side_effect_is_parsed`: some side effect in atomic chess (`specs/atomic.json`) has `kind == SideEffectKind::Explode`.
  - `win_condition_is_parsed`: `specs/king_of_the_hill.json` has `win_conditions[0].kind == WinConditionKind::PieceInZone`.
  - `declared_condition_type_is_parsed`: chess `conditions["REACH_END"].kind == CellConditionKind::Position`.
- [ ] **Step 2: Run** `cargo test -p modern-chess-engine rule_codes` — Expected: compile errors naming the missing enums/fields.
- [ ] **Step 3: Implement** the enums and parse at build; dispatch sites `match` on `kind` with no `From<&str>` at evaluation time. Respell the session's `def.r#type == POSITION` to `def.kind == CellConditionKind::Position`.
- [ ] **Step 4: Run** `cargo test -p modern-chess-engine` — Expected: all pass (322 + 5 new, 1 ignored). Then `cargo build --workspace` — Expected: 0 warnings. `grep -rn "From<&str>" crates/engine/src/logic` — Expected: only the `From` impls, called from build code (no call inside `evaluate_condition`, `apply_side_effect`, `check_win_conditions`).
- [ ] **Step 5: Commit** `refactor(engine): parse rule codes into enums when the game is built`

### Task 2: Rules behind an `Arc`

**Files:**
- Modify: `crates/engine/src/logic/game/main.rs` (new `Rules`, `Game { rules, board, turn_order, state }`)
- Modify: every engine file reading `self.<rule field>` / `game.<rule field>` (71 sites), engine tests (7 assignments, ~24 reads)
- Modify: `crates/session/src/session.rs`, `crates/game/src/**`, `apps/cli/src/main.rs` where they read rule fields
- Test: `crates/engine/src/tests/rules_shared/mod.rs` (new)

**Interfaces:**
- Consumes: `WinCondition` from Task 1.
- Produces: `pub struct Rules { pub name, pub conditions, pub players, pub blueprints, pub leader, pub repetition_count, pub fifty_move_halfmoves, pub fifty_move_pawn_codes, pub insufficient_material, pub stalemate_loses, pub hand_enabled, pub piece_values, pub demotes_to, pub win_conditions, pub players_with_configured_leader, pub no_opponent_check, pub jump_enabled, pub mandatory_capture, pub allow_pass, pub forbid_repeated_positions, pub liberty_capture, pub unlimited_supply }` (same types as today's fields); `Game { pub rules: Arc<Rules>, pub board: Board, pub turn_order: Vec<String>, pub state: GameState }` (Task 3 then moves `turn_order` into state).

- [ ] **Step 1: Write the failing tests:**
  - `a_cloned_game_shares_its_rules`: `Arc::ptr_eq(&game.rules, &game.clone().rules)`.
  - `changing_a_clones_rules_leaves_the_original`: `let mut copy = game.clone(); Arc::make_mut(&mut copy.rules).allow_pass = !game.rules.allow_pass;` then `copy.rules.allow_pass != game.rules.allow_pass`.
  - `eliminating_a_player_changes_only_that_game`: four-player chess, clone, eliminate on the clone through the public path the existing elimination tests use; the original's `turn_order.len()` is unchanged.
- [ ] **Step 2: Run** `cargo test -p modern-chess-engine rules_shared` — Expected: compile error, no field `rules`.
- [ ] **Step 3: Implement** `Rules` and respell call sites mechanically (`self.hand_enabled` → `self.rules.hand_enabled`; test assignments → `Arc::make_mut(&mut game.rules).x = ..`). `Rules` derives `Debug, Clone`.
- [ ] **Step 4: Run** `cargo test --workspace` (DB container up) — Expected: engine 330 pass + 1 ignored, session/wasm 80, game 160; `cargo build --workspace` 0 warnings.
- [ ] **Step 5: Commit** `refactor(engine): keep the rules a game is built with behind one Arc`

### Task 3: The turn order is saved

**Files:**
- Modify: `crates/engine/src/logic/game/structs/state.rs` (`GameState.turn_order: Vec<String>`, `#[serde(default)]`)
- Modify: `crates/engine/src/logic/game/main.rs` (`Rules.turn_order` = the spec's order; `Game.turn_order` removed; state starts with a copy)
- Modify: `crates/engine/src/logic/game/io.rs` (`restore_state`: an empty `turn_order` is filled from `rules.turn_order`)
- Modify: every `self.turn_order` / `game.turn_order` (engine, tests, `crates/session/src/session.rs:56`) → `state.turn_order`
- Test: `crates/engine/src/tests/game_over/mod.rs` (two new tests beside the elimination tests)

**Interfaces:**
- Consumes: `Rules` from Task 2.
- Produces: `GameState { pub turn_order: Vec<String>, .. }`; `Rules { pub turn_order: Vec<String>, .. }`; `Game { rules, board, state }`.

- [ ] **Step 1: Write the failing tests** in `tests/game_over/mod.rs`, reusing `load_game_3player` and the board set-up of `test_nplayer_checkmated_player_is_eliminated`:
  - `test_elimination_survives_save_and_restore`: after WHITE is eliminated, `let saved = game.save_state()?;` restore into a fresh `load_game_3player()`; assert the restored game's turn order has no "WHITE", has length 2, and its current player equals the original's.
  - `test_state_saved_without_turn_order_uses_the_spec_order`: take `save_state()` of a fresh game, remove the `turn_order` key from the JSON, restore; assert the turn order equals `["BLACK", "WHITE", "RED"]`.
- [ ] **Step 2: Run** `cargo test -p modern-chess-engine game_over` — Expected: the first test FAILS (restored order still has WHITE); the second fails to compile until the field exists — write it against `state.turn_order` so both fail at compile first, then the first fails on assertion after the field is added with no restore fallback.
- [ ] **Step 3: Implement** the field, the move of `Game.turn_order` into state, and the fallback in `restore_state`.
- [ ] **Step 4: Run** `cargo test --workspace` (DB container up) — Expected: all pass, including `stored_state` unchanged; 0 warnings.
- [ ] **Step 5: Commit** `fix(engine): save the turn order with the game state`

### Task 4: Modules grouped by job

**Files:** (all moves with `git mv`)
- `logic/game/main.rs` → `logic/game/mod.rs` content (Game, Rules); module declarations stay in `mod.rs`
- `logic/game/structs/{board,graph}.rs` → `logic/game/board/{mod,graph}.rs`
- `logic/game/structs/{state,piece,move_record}.rs` → `logic/game/state/{mod,piece,move_record}.rs`
- `logic/game/enums/{phase,transition,error}.rs` → `logic/game/{phase,transition,error}.rs`
- `logic/game/structs/condition.rs` + `logic/game/conditions.rs` → `logic/game/cell_conditions.rs`
- `logic/blueprint/move_blueprint.rs` → `logic/blueprint/move_blueprint/{mod.rs (types), build.rs (from_spec and its helpers), generate.rs (calculate_*, check_conditions, helpers)}`
- `lib.rs`: `#[cfg(test)] mod tests;`
- Replace glob re-exports (`pub use game::*`, `structs::*`, `enums::*`) with named ones; internal modules `pub(crate)`.

**Interfaces:**
- Consumes: Tasks 1–3.
- Produces: the same public names at `modern_chess::logic::*`, `modern_chess::shared::*`, `modern_chess::specs::*` that the other crates import today.

- [ ] **Step 1: Move files and fix paths.** No new test: this task changes no behaviour and adds no function; the suite and the other crates' builds are its check (ledger a ruling saying so).
- [ ] **Step 2: Run** `cargo build --workspace` — Expected: 0 errors, 0 warnings (no unused or private-in-public warnings). `grep -rn "pub use .*::\*" crates/engine/src/logic` — Expected: no output. `cargo build -p modern-chess-wasm --target wasm32-unknown-unknown` — Expected: success.
- [ ] **Step 3: Run** `cargo test --workspace` — Expected: same counts as Task 3. `cargo fmt --check -p modern-chess-engine` — Expected: no output.
- [ ] **Step 4: Commit** `refactor(engine): group the game modules by job and re-export by name`
