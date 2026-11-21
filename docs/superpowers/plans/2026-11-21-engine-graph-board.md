# The Board Is a Graph Inside the Engine: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The engine walks a graph of named positions instead of adding coordinates, while every game, every stored spec and every stored match behaves and loads exactly as before.

**Architecture:** A grid spec compiles into a graph when the game is built. Each square becomes a node named after its coordinates (`"4,2"`). Every vector the rules use becomes a labelled edge table. The board also holds two neighbourhoods, `orthogonal` and `adjacent`. Blueprints keep, for each player, the label of each step and offset. Each of these maps comes from that player's orientation matrix. Moves, conditions, side effects, liberties and scores follow edges and neighbourhoods. `Position` becomes a node name. It is written to JSON in exactly the shape stored states already use.

**Tech Stack:** Rust (`modern-chess-engine`, `modern-chess-session`), serde, std `Arc` and `OnceLock`.

**Spec:** `docs/superpowers/specs/2026-11-21-engine-generalisation-design.md`, section "Step 2: the board is a graph inside the engine", plus "Calls made after review".

## Global Constraints

- Any change to how an existing game plays is ruled out. Every game in `crates/engine/specs` behaves exactly as before at every step. The engine's test suite passes unchanged: tests may be added, and test helpers may change shape, but no expectation changes.
- Assertions never change. Call sites may be respelled mechanically when a public type changes (for example `vec![3, 0]` becoming a position name).
- Stored data is immutable. The engine keeps reading every spec and every state it has ever written. The JSON of a grid game's spec and state does not change shape.
- The engine never panics outside tests: `lib.rs` denies `unwrap`, `expect` and indexing.
- Nothing a person or a spec sees changes. The session's public API and its `View` stay identical.
- One code path: after Task 3 no rule adds coordinates. Grids exist only in the compiler that builds the graph.
- The proof: the full engine suite passes unchanged, and a timing check shows that move generation for chess and 19×19 Go is not noticeably slower. The before and after numbers go in the PR.

## Decisions (the builder's, from "Left to the builder")

- **Nodes:** a `NodeId` (`u32`) indexes `Vec`s inside the graph. Nodes are numbered in the order `all_positions` used before: lexicographic, first coordinate outermost, disabled squares skipped. Sorting by `NodeId` therefore gives the old coordinate order wherever order shows: repetition keys, liberty removals and explosion clears.
- **Labels:** a `LabelId` (`u32`). A grid label is the vector it moves by. Labels are collected while the blueprints are built, by a `Labels` interner, so the board builds a table for every label the rules use and for no other. Interning a vector also interns its unit vector (each component's sign), which is the label a path walks.
- **Per-player label maps:** `Labels::for_player(direction, vector)` turns a spec vector by the player's orientation and interns it. Step 3 will put a graph board's spec-given label map at this same point.
- **Sharing:** the graph sits behind an `Arc`, so cloning a `Game` (Atomic's threat check does) does not copy edge tables. The neighbourhoods are built on first use (`OnceLock`), so a board whose rules never use them pays nothing.
- **Paths:** a line from A to B walks B's unit label from A until it reaches B. When the walk ends without reaching B, the path does not exist. For every vector a shipped spec uses (all straight lines: castling, cannon screens, the xiangqi elephant), this gives the same squares as before.
- **Names in JSON:** a `Position` whose name is comma-separated canonical `u8`s serializes as an array (`[4, 2]`), the empty name as `[]`, and any other name as a string. Deserializing accepts an array or a string. The keys of `GameState.pieces` stay `"x,y"` strings, and a key made of `u8`s is canonicalised as the old reader did (`" 4, 2"` loads as `"4,2"`).

## Review Focus

1. **A match stored before this change** is the most likely to break. Its pieces are keyed `"x,y"`, its history and phase hold arrays, and its repetition keys are strings. It must restore, keep counting toward repetition and save back in the same shape. Task 1 pins this with a fixture written by today's engine.
2. **Boards with disabled squares:** a slide stops at a hole, a leap lands beyond it, and an explosion leaves holes out. No shipped spec has holes, so nothing else exercises this. Task 3 adds `holes_stop_slides_but_not_leaps`.
3. **The largest legal board** (100×100) with sliding pieces must build and play without panicking. Task 2 adds `builds_and_walks_a_ten_thousand_square_board`.
4. **A piece placed outside the board** by a spec's starting positions (not checked today) must not panic. Task 3 adds `a_piece_off_the_board_has_no_moves`. This is a ruling, since such a piece could slide back onto the board before.
5. **Non-straight paths** (`PATH_PIECE_COUNT` or `PATH_NOT_ATTACKED` on a vector like `[2, 1]`) now fail instead of counting the stair-step squares the old arithmetic visited. No shipped spec does this. Task 3 adds `a_path_that_is_not_a_line_does_not_exist`, and it is ledgered as a ruling.

---

### Task 1: Baseline: stored-state fixture and timing

**Files:**
- Create: `crates/engine/src/tests/stored_state/mod.rs`, `crates/engine/src/tests/stored_state/chess_midgame.json`
- Create: `crates/engine/src/tests/timing/mod.rs`
- Modify: `crates/engine/src/tests/mod.rs` (add `mod stored_state; mod timing;`)

**Interfaces:**
- Produces: the fixture `chess_midgame.json`, a `GameState` saved by the engine as it is today (on main). It comes from the chess spec after 1.e4 e5 2.Nf3 Nc6 3.Ng1 Nb8 4.Nf3, then the white bishop on f1 picked up (`CalculateMoves` at `[5, 0]`). So it holds a `Moving` phase, a 7-move history and position hashes with one repetition.

- [ ] **Step 1: Write the fixture generator as a one-off ignored test, run it on the current engine, and save its output to `chess_midgame.json`**, pretty-printed with `serde_json::to_string_pretty` of `serde_json::from_str::<Value>(&game.save_state())`. Delete the generator afterwards. The fixture is the record.
- [ ] **Step 2: Write the tests in `stored_state/mod.rs`**:

```rust
#[test]
fn a_state_saved_before_the_graph_loads_and_saves_unchanged() {
    // restore fixture into a chess game, save again, compare as serde_json::Value
    assert_eq!(saved_again, fixture);
}
#[test]
fn a_restored_state_keeps_its_repetition_keys() {
    // after restore: game.position_key() == last entry of fixture["position_hashes"]
}
#[test]
fn a_restored_state_plays_on_and_counts_the_repetition() {
    // restore, then keep shuffling the knights (4...Nc6 5.Ng1 Nb8 6.Nf3 ...): the game is
    // still running one move before the third occurrence and is GameOver { winner: None } on it
}
```

(The third test's exact move count is read off today's engine when the test is written. A repeat counts only once both knights have moved, because the key records whether each piece has moved. The test must pass before any engine change.)

- [ ] **Step 3: Run them.** Run `cargo test -p modern-chess-engine stored_state`. Expected: 3 passed (they describe today's behaviour).
- [ ] **Step 4: Write the timing test `timing/mod.rs`** as `#[test] #[ignore] fn move_generation_timing()`. It prints the mean time per call, over 200 runs each, for:
  - (a) chess from the opening: `CalculateMoves` for every White piece, plus `any_legal_moves()`;
  - (b) a 19×19 Go position with 60 stones placed alternately by drops: `CalculateDrops` for a stone, plus `any_legal_moves()`.
- [ ] **Step 5: Run it in release mode and record the numbers in the ledger.** Run `cargo test --release -p modern-chess-engine timing -- --ignored --nocapture`. Expected: two lines of timings.
- [ ] **Step 6: Commit** `test(engine): pin a stored chess state and time move generation`.

### Task 2: The graph and the grid compiler

**Files:**
- Create: `crates/engine/src/logic/game/structs/graph.rs` (`Graph`, `NodeId`, `LabelId`, `Labels`, `Offset`)
- Modify: `crates/engine/src/logic/game/structs/board.rs` (the `Board` wraps an `Arc<Graph>`; `all_positions`, `positions_within` and `is_position_valid` answer from the graph)
- Modify: `crates/engine/src/logic/game/structs/mod.rs`
- Create: `crates/engine/src/tests/board_graph/mod.rs`

**Interfaces:**
- Produces, in `graph.rs`:
  - `pub type NodeId = u32; pub type LabelId = u32;`
  - `#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct Offset { pub label: LabelId, pub unit: LabelId }`
  - `#[derive(Default, Clone, Debug)] pub struct Labels`, with:
    - `pub fn intern(&mut self, vector: &ExtendedPosition) -> Offset` (interns the vector and its unit);
    - `pub fn for_player(&mut self, direction: &Direction, vector: &ExtendedPosition) -> Offset` (applies `apply_direction` first);
    - `pub fn vectors(&self) -> &[ExtendedPosition]` (indexed by `LabelId`).
  - `Graph` (private fields), built by `Graph::grid(spec: &BoardSpec, labels: &Labels) -> Graph`.
- Produces, on `Board` (the graph's answers, in names; `Position` is still `Vec<u8>` in this task):
  - `pub fn new(spec: BoardSpec, labels: &Labels) -> Board`. `from_spec(spec)` is kept as `new(spec, &Labels::default())`.
  - `pub fn node(&self, position: &Position) -> Option<NodeId>`
  - `pub fn name(&self, node: NodeId) -> Option<&Position>`
  - `pub fn follow(&self, node: NodeId, label: LabelId) -> Option<NodeId>`
  - `pub fn step(&self, position: &Position, label: LabelId) -> Option<&Position>`
  - `pub fn orthogonal(&self, node: NodeId) -> &[NodeId]`. Order: axis 0 −1, axis 0 +1, axis 1 −1, … (today's order).
  - `pub fn adjacent(&self, node: NodeId) -> &[NodeId]`
  - `pub fn path(&self, from: NodeId, to: NodeId, unit: LabelId) -> Option<Vec<NodeId>>`. It returns the nodes after `from` up to and including `to`: `Some(vec![])` when `from == to`, and `None` when the walk ends, or runs longer than the node count, without reaching `to`.
  - `pub fn coordinates(&self, position: &Position) -> Option<&[u8]>`
  - `pub fn order(&self, position: &Position) -> Option<NodeId>`, the sort key (callers put unknown names last).
  - `all_positions()` returns names in node order.
  - `positions_within(&Position, radius)` gives `radius` rounds of `adjacent` from the centre, centre included, sorted by node.
  - `is_position_valid(&ExtendedPosition)` is true when the coordinates name a node.

- [ ] **Step 1: Write the failing tests in `board_graph/mod.rs`** (`Board::new` on a `BoardSpec` literal, with `Labels` built by hand):

```rust
#[test] fn names_grid_squares_by_their_coordinates_in_board_order() {
    // 3×2 board with [1,1] disabled
    assert_eq!(board.all_positions(), vec![vec![0,0], vec![0,1], vec![1,0], vec![2,0], vec![2,1]]);
    assert_eq!(board.node(&vec![1,1]), None);
}
#[test] fn follows_a_label_and_stops_at_the_edge_and_at_holes() {
    // labels {[1,0]} on the board above: "0,0"→"1,0"; "2,0"→None; "0,1"→None (hole at 1,1)
}
#[test] fn a_player_label_is_the_vector_turned_by_their_orientation() {
    // for_player([[-1,0],[0,-1]], [0,1]) == intern([0,-1]); its unit == intern([0,-1]).label
}
#[test] fn orthogonal_and_adjacent_neighbourhoods_on_a_grid() {
    // 3×3: orthogonal("1,1") names == ["0,1","2,1","1,0","1,2"]; adjacent("1,1").len()==8;
    // adjacent("0,0").len()==3; on a 3×3×3 board adjacent of the centre has 26
}
#[test] fn walks_a_straight_line_between_two_squares() {
    // 8×8 with labels {[1,0]}: path("0,0"→"3,0", unit [1,0]) == ["1,0","2,0","3,0"];
    // path("0,0"→"3,1", unit [1,0]) == None; path(a→a) == Some([])
}
#[test] fn builds_and_walks_a_ten_thousand_square_board() {
    // 100×100 with labels for the 8 king vectors: follow along [1,1] from "0,0" 99 times reaches "99,99"
}
```

- [ ] **Step 2: Run them.** Run `cargo test -p modern-chess-engine board_graph`. Expected: compile errors (`Labels`, `Board::new` missing).
- [ ] **Step 3: Implement `graph.rs` and the `Board` methods above.**
  - Nodes come from the existing odometer.
  - A label table is a `Vec<NodeId>` per label, with `u32::MAX` meaning "no edge". It is filled by adding the vector to each node's coordinates and looking the result up.
  - The neighbourhoods are `OnceLock<Vec<Vec<NodeId>>>`, filled from coordinates on first use.
  - `Board` derives `Clone` cheaply through `Arc<Graph>`. `dimensions` and `disabled_positions` stay public fields; the session draws from them.
- [ ] **Step 4: Run the whole suite.** Run `cargo test -p modern-chess-engine`. Expected: all pass, including `board_shape`, which now answers from the graph.
- [ ] **Step 5: Commit** `feat(engine): compile grid boards into graphs of named squares`.

### Task 3: The engine walks the graph

**Files:**
- Modify: `logic/blueprint/move_blueprint.rs`. `step` becomes `HashMap<String, Offset>`, and so do `Condition.position` and the side-effect `from`/`to`/`target`. `from_spec` takes `&mut Labels`. Slides and threats walk `follow` by node.
- Modify: `logic/blueprint/piece_blueprint.rs` (`from_spec(spec, players, &mut Labels)`)
- Modify: `logic/game/main.rs`. `from_spec` builds the blueprints first, collecting labels, then `Board::new(spec.board, &labels)`.
- Modify: every rule under `logic/blueprint/conditions/` and `logic/blueprint/side_effects/`. Each uses `board.step(source, offset.label)` in place of adding vectors. `PATH_PIECE_COUNT` and `PATH_NOT_ATTACKED` use `board.path(...)` with the unit label.
- Modify: `logic/game/liberties.rs` and `scoring.rs` (`orthogonal`), `explode.rs` and `ally_adjacent_count.rs` (`adjacent`)
- Modify: `shared/position.rs`. Remove `into_position`, `into_extended_position` and the rule-side arithmetic helpers once unused.
- Test: `crates/engine/src/tests/graph_rules/mod.rs`

**Interfaces:**
- Consumes: Task 2's `Labels`, `Offset`, `Board::{new, node, name, follow, step, path, orthogonal, adjacent, order}`.
- Produces: no new public API. After this task, `grep -rn "as i16 +" crates/engine/src/logic` finds nothing.

- [ ] **Step 1: Write the failing tests in `graph_rules/mod.rs`.** Use small `serde_json::json!` specs, like `three_d`.

```rust
#[test] fn holes_stop_slides_but_not_leaps() {
    // 5×1 board, disabled [2,0]. White ROOK at [4,0] (step [-1,0], repeat):
    // destinations == {[3,0]}. White LEAPER at [1,0] (step [2,0]): destinations == {[3,0]}.
}
#[test] fn an_explosion_leaves_holes_alone_and_reaches_past_them() {
    // 3×3 board, disabled [1,0]; capture on [1,1] with EXPLODE radius 1 clears all 7 other squares' pieces
}
#[test] fn a_piece_off_the_board_has_no_moves() {
    // starting position [8,0] on an 8×8 board: CalculateMoves at [8,0] gives no destinations, no panic
}
#[test] fn a_path_that_is_not_a_line_does_not_exist() {
    // PATH_PIECE_COUNT max 0 on step [2,1] with an empty board: the move is not offered
}
```

- [ ] **Step 2: Run them.** Run `cargo test -p modern-chess-engine graph_rules`. Expected: `a_piece_off_the_board_has_no_moves` and `a_path_that_is_not_a_line_does_not_exist` fail (today's arithmetic behaves differently). The hole tests may already pass. Record which in the ledger, since they pin behaviour that must survive.
- [ ] **Step 3: Implement the switch.**
  - Blueprints intern through `labels.for_player(&player.direction, &vector)`.
  - Any rule that took a target "off the board" now reads `None` from `board.step`. The rule's existing off-board branch is kept as it is (`LEG_EMPTY` passes, `PIECE_AT` fails, and so on).
  - `PATH_PIECE_COUNT` counts the pieces on the path minus its last node. It uses the offset's unit when given, otherwise the step's unit. Its target must exist and differ from the source.
  - `PATH_NOT_ATTACKED` needs every node of `path(source, step target, step unit)` unattacked. It passes for a zero step.
  - `liberties.rs` sorts removals by `board.order`.
- [ ] **Step 4: Run the whole suite.** Run `cargo test -p modern-chess-engine && cargo test -p modern-chess-session`. Expected: all pass, with no test edited except the new file.
- [ ] **Step 5: Commit** `feat(engine): moves and rules follow labelled edges of the board graph`.

### Task 4: Positions are node names

**Files:**
- Modify: `shared/position.rs`:
  - `pub type Coordinates = Vec<u8>;`
  - `#[derive(Clone, Debug, PartialEq, Eq, Hash)] pub struct Position(String)`, deliberately with no `Ord`: order comes from the board.
  - `Position::from_coordinates(&[u8])`, `as_str()`, `Borrow<str>`, `From<&str>`, `From<String>`, `Display`.
  - Serde as in Decisions.
  - `#[macro_export] macro_rules! pos` with `pos![4, 2] == Position::from_coordinates(&[4, 2])`.
- Modify: `specs/game/board.rs`, `player.rs` and `condition.rs`, which use `Coordinates` for spec data (JSON unchanged).
- Modify: `logic/game/structs/state.rs`. The pieces map serde writes names and reads them canonicalised.
- Modify: `logic/game/draw.rs`. `position_key_for` sorts by `board.order`, then name, and prints the name.
- Modify: `logic/game/legal.rs`. `ALLY_ON_FILE` compares `board.coordinates(..).first()`.
- Modify: `logic/game/conditions.rs`. A `POSITION` condition compares the name.
- Modify: `logic/game/handlers/execute_drop.rs` and `pass.rs`. The empty `from`/`to` are `Position::from("")`.
- Modify: `crates/session/src/session.rs`. It builds `Position::from_coordinates(&[x, y])` and reads squares through `game.board.coordinates`.
- Modify (respelling only): engine tests, session tests.
- Test: add to `board_graph/mod.rs`.

**Interfaces:**
- Consumes: Task 2's `Board::{coordinates, order}`.
- Produces: `Position` as a name everywhere in the engine API; `Coordinates` in spec types; `pos![..]`.

- [ ] **Step 1: Write the failing tests** in `board_graph/mod.rs`:

```rust
#[test] fn grid_names_are_written_as_coordinates() {
    assert_eq!(serde_json::to_value(pos![4, 2]).unwrap(), json!([4, 2]));
    assert_eq!(serde_json::to_value(Position::from("")).unwrap(), json!([]));
    assert_eq!(serde_json::to_value(Position::from("north")).unwrap(), json!("north"));
    assert_eq!(serde_json::from_value::<Position>(json!([4, 2])).unwrap(), Position::from("4,2"));
    assert_eq!(serde_json::from_value::<Position>(json!("north")).unwrap(), Position::from("north"));
}
#[test] fn a_name_that_only_looks_like_coordinates_stays_a_string() {
    assert_eq!(serde_json::to_value(Position::from("04,2")).unwrap(), json!("04,2"));
}
```

- [ ] **Step 2: Run them.** Run `cargo test -p modern-chess-engine board_graph`. Expected: compile errors (`pos!` and `Position::from` missing).
- [ ] **Step 3: Implement the type change in the engine and the session.** Then respell the call sites the compiler flags:
  - a `vec![..]` that stands for a runtime position becomes `pos![..]`;
  - `&vec![..]` becomes `&pos![..]`;
  - a literal `"x,y"` is allowed where it reads better (`pieces.get("6,0")`).

  No assertion's meaning changes. Spec data in tests (`Coordinates`, steps) stays `vec![..]`.
- [ ] **Step 4: Run every suite.** Run `cargo test -p modern-chess-engine && cargo test -p modern-chess-session && cargo test -p modern-chess-wasm && cargo build --workspace`, then the `modern-chess-game` and `modern-chess-game-api` tests against `modern-chess-db`. Expected: all pass. `stored_state` must pass untouched: it is the proof that stored matches load.
- [ ] **Step 5: Measure.** Run the release timing from Task 1. Expected: both figures within noise of the baseline (≤ 10% slower), recorded in the ledger next to the baseline. If one is slower, profile and fix within this task before committing.
- [ ] **Step 6: Commit** `feat(engine): positions are node names, written in the stored shape`.
