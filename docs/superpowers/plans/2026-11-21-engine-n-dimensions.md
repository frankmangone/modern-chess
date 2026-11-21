# Engine N Dimensions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the places where the engine assumes a 2D board, so a spec on a board of any number of dimensions loads, is checked, and plays correctly.

**Architecture:** The orientation matrix and the relative offsets become vectors sized by the board's number of dimensions. The spec checker refuses any matrix, step or offset of the wrong size, and boards too large to list. `Board` lists positions and neighbourhoods on every axis, and `EXPLODE` and `ALLY_ADJACENT_COUNT` use that neighbourhood. No 2D game changes behaviour, and no spec's JSON changes.

**Tech Stack:** Rust (crate `modern-chess-engine`, lib name `modern_chess`), serde, serde_json.

**Spec:** `docs/superpowers/specs/2026-11-21-engine-generalisation-design.md`, "Step 1: remove the remaining 2D limits" and "Calls made after review".

## Global Constraints

- No game in `crates/engine/specs` changes behaviour. Every existing test assertion stays as it is. Call sites may only be respelled where a type changes (`[[1, 0], [0, 1]]` becoming `vec![vec![1, 0], vec![0, 1]]`).
- No spec JSON changes shape. `"direction": [[1, 0], [0, 1]]` and `"position": [1, 0]` keep parsing.
- Outside tests the engine never panics: `crates/engine/src/lib.rs` denies `unwrap`, `expect` and indexing. Use `get`, `zip` and early returns.
- Board limits: refuse a board with no dimensions, any dimension of size 0, more than 8 dimensions, or more than 10,000 positions.
- The session, wasm and board element stay 2D. 3D is engine-only in this step.
- Conventional commits, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. A proposed spec asking for a huge or many-dimensional board must be refused by the checker, not hang listing positions (Task 1: `rejects_boards_that_are_empty_or_too_large`).
2. A 2×2 orientation on a 3D board must be refused. Without that, `apply_direction` would silently drop the third coordinate (Task 1: `rejects_a_direction_of_the_wrong_size`).
3. A matrix with large entries must not overflow while its determinant is computed. Overflow counts as "not a rotation" (Task 1: `determinant_reports_overflow_as_none`).
4. Every shipped spec must still parse after the type changes, including the ones no game test loads (`4player_chess`, `solidarity`, `janggi_cannon`, `mini_xiangqi`) (Task 1: `accepts_every_shipped_spec`).
5. 2D boards must list positions in the same order as before, since drops and scoring walk that list (Task 2: `lists_2d_positions_in_the_same_order_as_before`).

---

### Task 1: Orientation and offsets of any size, checked against the board

**Files:**
- Modify: `crates/engine/src/shared/position.rs` (`Direction`, new `determinant`)
- Modify: `crates/engine/src/specs/game/piece/condition.rs:14` (`position`)
- Modify: `crates/engine/src/specs/game/piece/side_effect.rs:24-30` (`from`, `to`, `target`)
- Modify: `crates/engine/src/logic/blueprint/move_blueprint.rs:134-171` (`transform_pos`, `build_conditions`)
- Modify: `crates/engine/src/specs/game/game.rs` (new error variants, `validate`)
- Modify: `crates/engine/src/specs/game/board.rs` (limits, `validate_size`)
- Modify: `crates/engine/src/specs/game/player.rs:56-66` (matrix check), `:490` (`from_name` test helper)
- Test: `crates/engine/src/tests/spec_str/mod.rs`, `crates/engine/src/tests/direction/mod.rs`

**Interfaces:**
- Produces:
  - `pub type Direction = Vec<Vec<i16>>;` (row-major, N×N).
  - `pub fn determinant(matrix: &Direction) -> Option<i128>`: `None` when the matrix is not square or the computation overflows. An empty matrix has determinant 1.
  - `ConditionSpec::position: Option<ExtendedPosition>` and `SideEffectSpec::{from, to, target}: Option<ExtendedPosition>`.
  - `GameSpecError::InvalidBoardSize(Vec<u8>)`.
  - `GameSpecError::MismatchedDimensions { what: String, expected: usize, found: usize }`, where `what` names the place, e.g. `"direction of player BLACK"` or `"step of move 0 of piece ROOK"`.
  - `GameSpecError::InvalidDirectionMatrix(Direction)`, unchanged in meaning.
  - In `specs/game/board.rs`: `pub const MAX_DIMENSIONS: usize = 8;`, `pub const MAX_POSITIONS: u64 = 10_000;` and `impl BoardSpec { pub fn validate_size(&self) -> Result<(), GameSpecError> }`.

- [ ] **Step 1: Write the failing tests**

In `tests/direction/mod.rs`:
- Respell the existing 2×2 literals as `vec![vec![..], vec![..]]`.
- Add these tests:

```rust
#[test]
fn rotates_a_step_in_three_dimensions() {
    let quarter_turn_about_z = vec![vec![0, -1, 0], vec![1, 0, 0], vec![0, 0, 1]];
    assert_eq!(apply_direction(&quarter_turn_about_z, &vec![1, 0, 0]), vec![0, 1, 0]);
    assert_eq!(apply_direction(&quarter_turn_about_z, &vec![0, 0, 1]), vec![0, 0, 1]);
}

#[test]
fn determinant_of_rotations_and_reflections() {
    assert_eq!(determinant(&vec![vec![1, 0, 0], vec![0, 1, 0], vec![0, 0, 1]]), Some(1));
    assert_eq!(determinant(&vec![vec![-1, 0, 0], vec![0, -1, 0], vec![0, 0, 1]]), Some(1));
    assert_eq!(determinant(&vec![vec![0, 1, 0], vec![1, 0, 0], vec![0, 0, 1]]), Some(-1));
    assert_eq!(determinant(&vec![vec![2, 0], vec![0, 1]]), Some(2));
    assert_eq!(
        determinant(&vec![vec![0, 0, 0, 1], vec![0, 0, 1, 0], vec![0, 1, 0, 0], vec![1, 0, 0, 0]]),
        Some(1)
    );
}

#[test]
fn determinant_of_a_matrix_that_is_not_square_is_none() {
    assert_eq!(determinant(&vec![vec![1, 0, 0], vec![0, 1, 0]]), None);
}

#[test]
fn determinant_reports_overflow_as_none() {
    let huge: Vec<Vec<i16>> = (0..8)
        .map(|row| (0..8).map(|col| if (row + col) % 2 == 0 { i16::MAX } else { i16::MIN }).collect())
        .collect();
    // Must return without panicking; the value is either exact or None.
    let _ = determinant(&huge);
}
```

In `tests/spec_str/mod.rs`, add the tests below. Each one mutates `CHATURANGA` with `serde_json::Value` the way `rejects_a_spec_with_nobody_to_move` does, and matches the error variant with `matches!`:
- `accepts_every_shipped_spec`: every `*.json` in `./specs` passes `parse_game_spec`. Iterate over `std::fs::read_dir("./specs")`.
- `rejects_a_direction_of_the_wrong_size`: `players[0].direction = [[1,0,0],[0,1,0],[0,0,1]]` gives `MismatchedDimensions { expected: 2, found: 3, .. }`.
- `rejects_a_direction_that_is_not_a_rotation`: `[[2,0],[0,1]]` gives `InvalidDirectionMatrix(_)`.
- `rejects_a_step_of_the_wrong_size`: `pieces[0].moves[0].step = [1,0,0]` gives `MismatchedDimensions { expected: 2, found: 3, .. }`.
- `rejects_an_offset_of_the_wrong_size`: run it three times:
  - `pieces[0].moves[0].conditions = [{"condition": "PIECE_AT", "position": [1,0,0]}]`;
  - `pieces[0].moves[0].side_effects = [{"action": "CAPTURE", "target": [0,1,0]}]`;
  - `pieces[0].moves[0].actions[0].side_effects = [{"action": "MOVE", "from": [0,0], "to": [1]}]`.
  Each gives `MismatchedDimensions`.
- `rejects_boards_that_are_empty_or_too_large`: `board.dimensions` set to `[]`, `[8, 0]`, `[101, 100]` and `[1,1,1,1,1,1,1,1,1]` each gives `InvalidBoardSize(_)`.
- `accepts_a_board_of_exactly_ten_thousand_positions`: `[100, 100]` parses.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p modern-chess-engine 2>&1 | tail -20`
Expected: compile errors, because `determinant`, the new variants and the `Vec` direction don't exist yet.

- [ ] **Step 3: Implement**

- `position.rs`:
  - change `Direction` to `Vec<Vec<i16>>`. `apply_direction` already iterates rows; keep its signature.
  - add `determinant` using fraction-free Gaussian elimination (Bareiss) on `i128`, with `checked_mul` / `checked_sub` / `checked_div`. Return `None` on any overflow. Swap rows on a zero pivot, flipping the sign each time. Return `Some(0)` when no pivot is found.
- `condition.rs` and `side_effect.rs`: change the four fields to `Option<ExtendedPosition>`.
- `move_blueprint.rs`: `transform_pos` and `build_conditions` pass the vector straight to `apply_direction` (no more `[r[0], r[1]]` unpacking). The `transform_pos` comment loses "[i8; 2]".
- `board.rs`, `BoardSpec::validate_size`: refuse with `InvalidBoardSize(dimensions.clone())` when:
  - `dimensions` is empty;
  - any dimension is 0;
  - `dimensions.len() > MAX_DIMENSIONS`;
  - the product of the dimensions, computed in `u64`, is over `MAX_POSITIONS`.
- `player.rs`:
  - The matrix must have `board.dimensions.len()` rows, each that long; otherwise return `MismatchedDimensions { what: format!("direction of player {}", self.name), expected, found }`. `found` is the row count, or the first wrong row length.
  - Then require `determinant == Some(1) || Some(-1)`, else `InvalidDirectionMatrix(self.direction.clone())`.
  - `from_name` builds the 2×2 identity with `vec!`.
- `game.rs`:
  - `validate` calls `self.board.validate_size()?` first.
  - Add `validate_vector_sizes`, run after `validate_steps`. It checks every move `step` and every offset against `board.dimensions.len()`, wherever a `ConditionSpec` or `SideEffectSpec` can sit:
    - move conditions;
    - action conditions;
    - action side effects, including a side effect's own `condition`;
    - move side effects;
    - modifier conditions;
    - `drop_restrictions`.
  - Update the `InvalidDirectionMatrix` message to "Direction is not a valid rotation matrix (det must be ±1): {0:?}" (it is unchanged; it just now prints a `Vec`).
- `specs/game/tests.rs`: `create_valid_game_spec` keeps compiling through `from_name`. Nothing else changes there.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p modern-chess-engine 2>&1 | grep -E "test result|FAILED|panicked"`
Expected: `test result: ok.`, with 278 + the new tests, and no failures.

- [ ] **Step 5: Commit**

```bash
git add crates/engine docs/superpowers/specs/2026-11-21-engine-generalisation-design.md docs/superpowers/plans/2026-11-21-engine-n-dimensions.md
git commit -m "feat(engine): orientations and offsets of any dimension, checked against the board"
```

### Task 2: Positions and neighbourhoods on every axis

**Files:**
- Modify: `crates/engine/src/logic/game/structs/board.rs`
- Modify: `crates/engine/src/logic/blueprint/side_effects/explode.rs:20-45`
- Modify: `crates/engine/src/logic/blueprint/conditions/ally_adjacent_count.rs:8-30`
- Create: `crates/engine/src/tests/board_shape/mod.rs`, registered in `crates/engine/src/tests/mod.rs`

**Interfaces:**
- Produces:
  - `Board::all_positions(&self) -> Vec<Position>`: every valid position on a board of any dimension, in lexicographic order with the first coordinate outermost. That is the current 2D order (column, then row).
  - `Board::is_position_valid(&self, position: &ExtendedPosition) -> bool`: also false when `position.len() != self.dimensions.len()`.
  - `Board::positions_within(&self, center: &Position, radius: u8) -> Vec<Position>`: every valid position within `radius` of `center` along every axis (Chebyshev distance), the centre included. Same order as `all_positions`. It is empty when `center` has the wrong length.

- [ ] **Step 1: Write the failing tests** in `tests/board_shape/mod.rs`, building boards with `Board::from_spec(BoardSpec { dimensions, disabled_positions })`:

```rust
#[test]
fn lists_every_position_of_a_3d_board() {
    let positions = board(vec![4, 4, 4], &[]).all_positions();
    assert_eq!(positions.len(), 64);
    assert_eq!(positions.first(), Some(&vec![0, 0, 0]));
    assert_eq!(positions.last(), Some(&vec![3, 3, 3]));
    let mut sorted = positions.clone();
    sorted.sort();
    assert_eq!(positions, sorted);
}

#[test]
fn lists_2d_positions_in_the_same_order_as_before() {
    assert_eq!(
        board(vec![2, 3], &[vec![1, 1]]).all_positions(),
        vec![vec![0, 0], vec![0, 1], vec![0, 2], vec![1, 0], vec![1, 2]]
    );
}

#[test]
fn refuses_positions_of_the_wrong_length() {
    let chess = board(vec![8, 8], &[]);
    assert!(chess.is_position_valid(&vec![3, 3]));
    assert!(!chess.is_position_valid(&vec![3]));
    assert!(!chess.is_position_valid(&vec![3, 3, 0]));
}

#[test]
fn neighbourhoods_reach_along_every_axis() {
    let cube = board(vec![4, 4, 4], &[]);
    assert_eq!(cube.positions_within(&vec![1, 1, 1], 1).len(), 27);
    assert_eq!(cube.positions_within(&vec![0, 0, 0], 1).len(), 8);
    assert_eq!(cube.positions_within(&vec![0, 0, 0], 2).len(), 27);
    assert!(cube.positions_within(&vec![1, 1], 1).is_empty());
    assert_eq!(board(vec![8, 8], &[vec![1, 1]]).positions_within(&vec![0, 0], 1).len(), 3);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p modern-chess-engine board_shape 2>&1 | tail -15`
Expected: a compile error, because `positions_within` is not defined.

- [ ] **Step 3: Implement**

- `all_positions`: an odometer over `dimensions` with the last coordinate turning fastest, skipping disabled positions.
- `positions_within`: the same odometer over each axis's range `center ± radius`, clamped to the board, skipping disabled positions.
- `is_position_valid`: compare lengths with `!=`, and check the disabled set once, after the bounds loop.
- `explode.rs`:
  - iterate `ctx.game.board.positions_within(ctx.target_position, radius)`;
  - the centre is the position equal to `ctx.target_position`;
  - drop the `[target_x, target_y, ..]` unpacking and the `dx`/`dy` loops;
  - keep the doc comment's rules, saying "along every axis".
- `ally_adjacent_count.rs`: iterate `positions_within(ctx.original_source, 1)` and skip the source itself. The comment says "surrounding positions on every axis" in place of "8 surrounding squares" (also in `shared/constants.rs:87`).

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p modern-chess-engine 2>&1 | grep -E "test result|FAILED|panicked"`
Expected: `test result: ok.` with no failures. The atomic and solidarity tests are what show 2D explosions and adjacency are unchanged.

- [ ] **Step 5: Commit**

```bash
git add crates/engine
git commit -m "feat(engine): list positions and neighbourhoods on boards of any dimension"
```

### Task 3: A 3D game proves it end to end

**Files:**
- Create: `crates/engine/src/tests/three_d/mod.rs`, registered in `crates/engine/src/tests/mod.rs`

**Interfaces:**
- Consumes:
  - `parse_game_spec_str` (`crate::specs`);
  - `Game::from_spec`, `GameTransition::{CalculateMoves, ExecuteMove}`, `GamePhase::GameOver`, `Piece::new`.
  - Tasks 1 and 2 must be in: the 3×3 orientation parses, and `positions_within` is used by `EXPLODE` and `ALLY_ADJACENT_COUNT`.

The spec is built in the test with `serde_json::json!` and parsed with `parse_game_spec_str`. It is not a file in `specs/`, which the game service seeds into the catalogue.
- Board `[4, 4, 4]`; `"leader": ["KING"]`; turns `["WHITE", "BLACK"]`.
- `WHITE` has the identity orientation; `BLACK` has `[[-1,0,0],[0,-1,0],[0,0,1]]`.
- `WHITE` starts with a `KING` at `[0,0,0]` and `BLACK` with a `KING` at `[3,3,3]`. Tests add other pieces through `game.state.pieces`, like the `insert` helper in `tests/atomic/mod.rs`.
- Pieces, writing `AXES` for the 6 unit steps and `ALL` for the 26 non-zero steps in `{-1,0,1}³`:
  - `ROOK`: one move per `AXES` step, actions `EMPTY→MOVE` and `ENEMY→CAPTURE`, `"repeat": {"until": "NOT_EMPTY", "loop": true}`.
  - `KING`: one move per `ALL` step, the same actions, `"conditions": [{"condition": "NOT_ATTACKED"}]`.
  - `BOMB`: one move per `AXES` step, the action `ENEMY→CAPTURE` with `"side_effects": [{"action": "EXPLODE"}]`.
  - `SCOUT`: one move with step `[1,0,0]`, the action `EMPTY→MOVE`, `"conditions": [{"condition": "ALLY_ADJACENT_COUNT", "min": 1}]`.

- [ ] **Step 1: Write the tests**
  - `a_rook_slides_along_all_three_axes`: a `WHITE` `ROOK` at `[1,1,1]`. Its sorted destinations are exactly the nine positions with two coordinates equal to 1 and the third in `{0, 2, 3}`.
  - `a_king_may_not_step_onto_a_line_attacked_along_the_third_axis`: a `BLACK` `ROOK` at `[1,1,3]`. The `WHITE` king's sorted destinations are exactly `[0,0,1], [0,1,0], [0,1,1], [1,0,0], [1,0,1]`.
  - `four_rooks_mate_a_cornered_king`:
    - `WHITE` `ROOK`s at `[0,2,2]`, `[0,2,3]`, `[0,3,2]`, `[1,0,3]`.
    - `CalculateMoves` at `[1,0,3]`, then `ExecuteMove` to `[1,3,3]`.
    - The phase is `GameOver { winner: Some("WHITE") }`.
  - `an_explosion_reaches_all_26_neighbours`:
    - Move the `BLACK` king from `[3,3,3]` to `[0,3,0]`.
    - Put a `WHITE` `BOMB` at `[2,2,1]`, and a `BLACK` `ROOK` on each of the other 26 positions of the cube `[1..=3]³`.
    - `CalculateMoves` at `[2,2,1]`, then `ExecuteMove` to `[2,2,2]`.
    - No position in `[1..=3]³` holds a piece. Both kings are still on the board.
  - `an_ally_one_step_away_on_the_third_axis_counts_as_adjacent`:
    - A `WHITE` `SCOUT` at `[1,1,1]` and a `WHITE` `ROOK` at `[2,2,2]`: the scout's destinations are `[[2,1,1]]`.
    - With the rook removed, it has none. Accept either the error or empty `available_moves` that `CalculateMoves` gives when a piece has no moves; check what `tests/atomic/mod.rs`'s `moves_for` does.

- [ ] **Step 2: Run the tests**

Run: `cargo test -p modern-chess-engine three_d 2>&1 | grep -E "test |test result"`
Expected: 5 passed. If one fails, the fault is in Tasks 1 to 2's code or in this test's setup. Find the root cause before changing anything, and never change an existing test's assertion.

- [ ] **Step 3: Commit**

```bash
git add crates/engine/src/tests
git commit -m "test(engine): a 3D game moves, checks, mates, explodes and counts neighbours"
```

### Task 4: Whole-workspace check

- [ ] **Step 1:** `cargo build --workspace 2>&1 | tail -3`. Expected: `Finished`. The session, wasm, game and CLI crates compile against the new types.
- [ ] **Step 2:** `cargo clippy -p modern-chess-engine 2>&1 | grep -E "^(error|warning: unused)" | head`. Expected: no output. The no-panic lints hold.
- [ ] **Step 3:** `cargo test -p modern-chess-engine -p modern-chess-session -p modern-chess-wasm 2>&1 | grep -E "test result|FAILED"`. Expected: all `ok`.
- [ ] **Step 4:** If the Postgres container `modern-chess-db` is up on 5455, run `cargo test -p modern-chess-game -p modern-chess-game-api`. Expected: all pass. The seed parses every shipped spec through the new checker. If it isn't up, say so in the PR instead of claiming the run.
