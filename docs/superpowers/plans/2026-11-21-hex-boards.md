# Hex Boards Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Specs can ask for a hex board (rhombus or hexagon), a general `CONNECTION` win condition exists, the board element draws hexes, and Hex and Gliński's hexagonal chess are playable and listed.

**Architecture:** The hex short form compiles, at parse time, into today's grid (`dimensions`, `disabled_positions`) plus a `BoardShape`; the graph picks the six hex vectors as its neighbourhoods for a hex board. `CONNECTION` is a new win-condition kind evaluated by a flood fill. The session's view carries `shape`/`layout`; the board element lays the same cell buttons out on hex centres. The two games are spec + page JSON.

**Tech Stack:** Rust workspace (`modern-chess-engine` lib `modern_chess`, `modern-chess-session`, `modern-chess-wasm`, `modern-chess-game`); vanilla JS custom element in `apps/docs/theme` (tests with `node --test`); TypeScript types in `apps/web`.

**Spec:** `docs/superpowers/specs/2026-11-21-hex-boards-design.md` (route: `docs/superpowers/specs/2026-11-21-engine-generalisation-design.md`, step 3)

## Global Constraints

- Hex neighbour vectors, exactly: `(0,1)`, `(0,-1)`, `(1,1)`, `(1,0)`, `(-1,0)`, `(-1,-1)`; on a hex board both `orthogonal` and `adjacent` are these six.
- Hex `rhombus` size *n* = the `n × n` grid; hex `hexagon` side *n* = the `(2n−1) × (2n−1)` grid with cells `|q − r| ≥ n` disabled.
- Square specs, stored commands (`{type, x, y}`) and saved states do not change.
- No one-game rule: `CONNECTION` stays general (zones, neighbourhood, `at_least`, `pieces`).
- The engine crate denies `unwrap`, `expect` and indexing outside tests and builds for wasm; workspace builds with 0 warnings; `cargo fmt` clean.
- Conventional commits ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Game crate tests need the `modern-chess-db` container.

## Review Focus

1. A square spec with no `shape` key, and stored specs in the database, parse and play exactly as before (`shape` defaults to square).
2. A hex spec that also gives `dimensions` is refused, not silently half-applied.
3. A chain touching one zone twice, or a zone touched only by the opponent's pieces, does not count; the win goes to the player who moved, never to the other one.
4. On a flipped (second-seat) board the hex a player clicks is the cell that gets played (click → `(x, y)` mapping survives the 180° turn and the rhombus rotation).
5. The game service seeds the two new specs and pages without breaking its catalogue tests (counts, families, preview variant by slug).

---

### Task 1: Hex boards in specs and in the graph

**Files:**
- Modify: `crates/engine/src/specs/game/board.rs` (hex short form, `BoardShape`, `HexLayout`)
- Modify: `crates/engine/src/logic/game/board/graph.rs` (neighbourhoods by shape), `crates/engine/src/logic/game/board/mod.rs` (`Board.shape`)
- Modify: `apps/docs/src/11-design-your-own.md` (the hex board form)
- Test: `crates/engine/src/tests/hex_board/mod.rs` (new; register in `tests/mod.rs`, sorted)

**Interfaces:**
- Produces:
  - `pub enum HexLayout { Rhombus, Hexagon }` (serde `"rhombus"`, `"hexagon"`), `pub enum BoardShape { Square, Hex { layout: HexLayout, size: u8 } }`, both `Clone, Copy, Debug, PartialEq, Eq`.
  - `BoardSpec { pub dimensions, pub disabled_positions, pub shape: BoardShape }` — after parsing a hex form, `dimensions` and `disabled_positions` hold the compiled grid (hexagon corners included in `disabled_positions`), so every existing reader keeps working.
  - `Board { pub shape: BoardShape, .. }`.
  - `Graph::grid` uses the six hex vectors for `orthogonal` and `adjacent` when `spec.shape` is `Hex`.

- [ ] **Step 1: Write the failing tests** in `tests/hex_board/mod.rs` (build games from inline JSON with one piece and no moves, via `parse_game_spec_str` + `Game::from_spec`):
  - `a_hexagon_of_side_6_has_91_cells`: `board.all_positions().len() == 91`; `pos![0, 10]` and `pos![10, 0]` are not on the board; `pos![5, 5]` is.
  - `a_rhombus_of_size_11_has_121_cells`.
  - `a_hex_cell_has_six_neighbours`: `board.orthogonal(&pos![5,5])` and `board.adjacent(&pos![5,5])` both equal, as sets, `{[5,6],[5,4],[6,6],[6,5],[4,5],[4,4]}`.
  - `rhombus_corners_have_two_or_three_neighbours`: `adjacent([0,0]).len() == 3`, `adjacent([10,10]).len() == 3`, `adjacent([10,0]).len() == 2`, `adjacent([0,10]).len() == 2`.
  - `a_hex_spec_with_dimensions_is_refused`: `{"shape":"hex","layout":"rhombus","size":5,"dimensions":[5,5]}` → `parse_game_spec_str` is `Err`.
  - `a_hex_size_of_zero_is_refused` → `Err`.
  - `a_square_spec_still_parses_unchanged`: `parse_game_spec("./specs/chess.json")` has `board.shape == BoardShape::Square` and `dimensions == [8, 8]`.
  - `disabled_positions_punch_holes_in_a_hexagon`: hexagon size 6 plus `"disabled_positions": [[5,5]]` → 90 cells.
- [ ] **Step 2: Run** `cargo test -p modern-chess-engine hex_board` — Expected: compile errors (`BoardShape` unknown).
- [ ] **Step 3: Implement.** Deserialize `BoardSpec` through a raw struct (`#[serde(try_from = "RawBoardSpec")]`, serialize back to the raw form): no `shape` or `"square"` → today's fields; `"hex"` → requires `layout` and `size ≥ 1`, refuses `dimensions`, compiles the grid; `size` whose grid exceeds `MAX_POSITIONS` is refused by the existing `validate_size`. `Board::new` copies `shape` from the spec.
- [ ] **Step 4: Run** `cargo test -p modern-chess-engine` — Expected: all pass (333 + 8). `cargo build --workspace --all-targets` — 0 warnings.
- [ ] **Step 5: Docs.** In `apps/docs/src/11-design-your-own.md`, next to `dimensions`, document `"shape": "hex"`, `layout`, `size`, the six neighbours, and that `disabled_positions` still applies.
- [ ] **Step 6: Commit** `feat(engine): hex boards, as a rhombus or a hexagon`

### Task 2: The `CONNECTION` win condition

**Files:**
- Modify: `crates/engine/src/specs/game/win_condition.rs` (`through`, `zones`, `at_least`; `pieces` doc extended)
- Modify: `crates/engine/src/specs/game/game.rs` (validation; new `GameSpecError::InvalidWinCondition(String)`)
- Modify: `crates/engine/src/shared/constants.rs` (`CONNECTION`, `ORTHOGONAL = "orthogonal"`, `ADJACENT = "adjacent"`)
- Create: `crates/engine/src/logic/game/connection.rs`; Modify: `logic/game/win.rs` (`WinConditionKind::Connection` arm), `logic/game/mod.rs`
- Modify: `apps/docs/src/05-winning-and-drawing.md`
- Test: `crates/engine/src/tests/connection/mod.rs` (new)

**Interfaces:**
- Consumes: hex neighbourhoods from Task 1 (tests also use a square board).
- Produces: `WinConditionSpec { pub through: Option<String>, pub zones: Vec<String>, pub at_least: Option<u8>, .. }`; `WinConditionKind::Connection`; `impl Game { pub(crate) fn connects(&self, player: &str, rule: &WinConditionSpec) -> bool }`.

- [ ] **Step 1: Write the failing tests** in `tests/connection/mod.rs`. Use an inline 5×5 square spec, players `A` and `B`, piece `STONE` (`unlimited_supply`), zones `LEFT` (A: `x = 0` cells, B: `y = 0` cells) and `RIGHT` (A: `x = 4`, B: `y = 4`), win condition `{"type":"CONNECTION","through":"orthogonal","zones":["LEFT","RIGHT"],"pieces":["STONE"]}`; place stones by inserting into `game.state.pieces`, then drop the last one through `CalculateDrops`/`ExecuteDrop` so the win check runs:
  - `a_chain_across_wins`: A stones on `(0..=4, 2)`, the last one dropped by A → `GameOver { winner: Some("A") }`.
  - `a_chain_one_short_does_not_win`: A stones on `(0..=3, 2)` → phase is not `GameOver`.
  - `a_diagonal_chain_does_not_connect_orthogonally`: A stones on `(i, i)` for `i` in `0..=4` → no win; the same position with `"through":"adjacent"` → A wins.
  - `the_opponents_sides_do_not_count`: A stones on `(2, 0..=4)` (B's zones, not A's) → no win.
  - `at_least_one_of_two_zones`: `"at_least": 1` and a single A stone on `(0, 2)` → A wins.
  - `unknown_zone_is_refused`, `unknown_neighbourhood_is_refused`, `at_least_zero_is_refused`, `at_least_above_the_zones_is_refused`: each `parse_game_spec_str` → `Err(GameSpecError::InvalidWinCondition(_))`.
- [ ] **Step 2: Run** `cargo test -p modern-chess-engine connection` — Expected: compile errors.
- [ ] **Step 3: Implement** `connects`: collect the player's pieces (filtered by `pieces` when non-empty), flood-fill each unvisited one through `board.orthogonal` / `board.adjacent`, and for each chain count the zones (named `POSITION` conditions, the player's own cell set) it touches; true when any chain reaches `at_least` (default: `zones.len()`). The `Connection` arm in `check_win_conditions` makes the player who moved the winner, like `PieceInZone`.
- [ ] **Step 4: Run** `cargo test -p modern-chess-engine` — Expected: all pass. `cargo build --workspace --all-targets` — 0 warnings.
- [ ] **Step 5: Docs.** Document `CONNECTION` in `apps/docs/src/05-winning-and-drawing.md` (fields, defaults, per-player zones), with Hex as the example.
- [ ] **Step 6: Commit** `feat(engine): win by connecting zones with a chain of pieces`

### Task 3: Hex

**Files:**
- Create: `crates/engine/specs/hex.json`, `crates/game/pages/hex.json`
- Test: `crates/engine/src/tests/hex_game/mod.rs` (new)

**Interfaces:**
- Consumes: Tasks 1–2.
- Produces: spec slug `hex`; zones `SIDE_A` (RED: `q = 0`, BLUE: `r = 0`) and `SIDE_B` (RED: `q = 10`, BLUE: `r = 10`).

- [ ] **Step 1: Write the failing tests** (load `./specs/hex.json`):
  - `hex_has_121_empty_cells_and_red_starts`: 121 positions, no pieces, `current_player() == "RED"`.
  - `red_wins_by_joining_its_sides`: RED stones on `(0..=9, 5)` inserted, then RED drops on `(10, 5)` → `GameOver { winner: Some("RED") }`.
  - `blue_wins_by_joining_its_sides`: BLUE stones on `(5, 0..=9)`, turn set to BLUE, BLUE drops on `(5, 10)` → BLUE wins.
  - `red_on_blues_sides_does_not_win`: RED stones on `(5, 0..=10)` → not `GameOver`.
  - `every_empty_cell_is_a_legal_drop`: on a fresh game, the drops for `STONE` cover all 121 cells.
- [ ] **Step 2: Run** `cargo test -p modern-chess-engine hex_game` — Expected: FAIL (spec file missing).
- [ ] **Step 3: Write `specs/hex.json`**: name `"HEX"`, board `{ "shape": "hex", "layout": "rhombus", "size": 11 }`, players `RED` and `BLUE` (identity direction, no starting pieces), turns `["RED","BLUE"]`, piece `STONE` with `unlimited_supply` and no moves, `leader: []`, zones `SIDE_A`/`SIDE_B` as `POSITION` conditions, win `{"type":"CONNECTION","through":"adjacent","zones":["SIDE_A","SIDE_B"],"pieces":["STONE"]}`.
- [ ] **Step 4: Run** the tests — Expected: PASS.
- [ ] **Step 5: Write `crates/game/pages/hex.json`**, modelled on `pages/go.json`: slug `hex`, name `Hex`, family `"Connection games"`, tags, accent, skin (red and blue stones as SVG discs; `zones` with per-player tints from Task 6's format: `SIDE_A`/`SIDE_B` → `{ "RED": <red tint>, "BLUE": <blue tint> }`), a how-to-play section that says: RED joins its two sides (top-right and bottom-left), BLUE the other two, a draw cannot happen, and the swap rule is not played yet.
- [ ] **Step 6: Run** `cargo test -p modern-chess-game` (DB up) — Expected: all pass; fix any catalogue test that counts pages or specs only by updating the expected count, and ledger it.
- [ ] **Step 7: Commit** `feat(games): Hex`

### Task 4: Gliński's hexagonal chess

**Files:**
- Create: `crates/engine/specs/glinski.json`, `crates/game/pages/glinski.json`
- Test: `crates/engine/src/tests/glinski/mod.rs` (new)

**Interfaces:**
- Consumes: Task 1. Notation helper in the test module: `fn cell(name: &str) -> Position` — file letter `a b c d e f g h i k l` → `q = 0..10`, rank *n* → `r = max(0, q − 5) + n − 1`.

- [ ] **Step 0: Check the starting position** against the setup diagram on Wikipedia's "Hexagonal chess" page (Gliński section) before writing the spec. Expected setup (design doc): White K g1, Q e1, R c1 i1, N d1 h1, B f1 f2 f3, P b1 c2 d3 e4 f5 g4 h3 i2 k1; Black K g10, Q e10, R c8 i8, N d9 h9, B f9 f10 f11, P b7 c7 d7 e7 f7 g7 h7 i7 k7. Ledger what was checked and how; if the diagram cannot be read, ledger that and flag it for Frank to confirm in "Try it".
- [ ] **Step 1: Write the failing tests** (load `./specs/glinski.json`; for move counts, clear the board and place one piece):
  - `the_board_has_91_cells_and_36_pieces`: 91 positions, 36 pieces (K, Q, 2 R, 2 N, 3 B, 9 P per side).
  - `starting_cells`: `cell("g1")` is a WHITE `KING`, `cell("e10")` a BLACK `QUEEN`, `cell("f5")` a WHITE `PAWN`, `cell("k7")` a BLACK `PAWN`.
  - `moves_from_the_centre`: a lone WHITE piece on `cell("f6")` (no kings: leader checks are skipped when a player has no leader on the board) has 30 moves as a rook, 12 as a bishop, 42 as a queen, 12 as a knight, 12 as a king.
  - `bishops_start_on_three_colours`: `(q + r) % 3` of `f1`, `f2`, `f3` are three different values.
  - `a_pawn_double_steps_from_a_borrowed_start`: a WHITE pawn on `cell("c2")` may move to `c3` and `c4`; a WHITE pawn placed on `cell("e4")` (another pawn's start) may move two; a WHITE pawn on `cell("e5")` may move only one.
  - `a_pawn_captures_at_sixty_degrees`: a WHITE pawn on `cell("f5")` = `(5,4)` with BLACK pawns on `(4,4)`, `(6,5)` and `(5,5)`: its moves are exactly `{(4,4), (6,5)}` (the two captures; straight ahead is blocked).
  - `en_passant`: BLACK pawn double-steps from `c7` to `c5` next to a WHITE pawn on `b5`; WHITE `b5` captures onto `c6` and the BLACK pawn on `c5` is removed (the design's example: "black c7–c5, then white b5xc6 e.p.").
  - `promotion_on_the_bent_edge`: a WHITE pawn on `cell("k6")` moving to `k7` is offered `QUEEN, ROOK, BISHOP, KNIGHT`.
  - `stalemate_is_a_win`: build a position where BLACK to move has no legal move, assert `!game.leader_in_check()` first, then make WHITE's last move into it → `GameOver { winner: Some("WHITE") }`.
- [ ] **Step 2: Run** `cargo test -p modern-chess-engine glinski` — Expected: FAIL (spec missing).
- [ ] **Step 3: Write `specs/glinski.json`** modelled on `specs/chess.json`: board `{ "shape": "hex", "layout": "hexagon", "size": 6 }`; WHITE identity direction, BLACK `[[-1,0],[0,-1]]`; leader `["KING"]`; `stalemate_loses: true`; no castling; draw conditions `repetition_count: 3`, fifty-move with pawn codes `["PAWN"]`, `insufficient_material: [["KING","KING"]]`; vectors exactly as the design's "Moves" list; the double step gated by a `POSITION` condition on its landing cells (WHITE: b3 c4 d5 e6 f7 g6 h5 i4 k3; BLACK: their 180° images) with the passed cell empty, setting chess's en-passant flag; promotion by a `POSITION` condition `REACH_END` (WHITE: a6 b7 c8 d9 e10 f11 g10 h9 i8 k7 l6; BLACK: a1 b1 c1 d1 e1 f1 g1 h1 i1 k1 l1).
- [ ] **Step 4: Run** the tests — Expected: PASS.
- [ ] **Step 5: Write `crates/game/pages/glinski.json`**, modelled on `pages/chess.json`: slug `glinski`, name "Gliński's hexagonal chess", family "Chess and its variants", walnut piece images, three wood tones (`squares.light`, `mid`, `dark`), how-to-play covering the board, each piece's move, the pawn's double step from any friendly pawn start, promotion at the end of each file, no castling, and "a stalemate wins for the player who causes it (Gliński scored it ¾–¼; the hub has no fractional results)".
- [ ] **Step 6: Run** `cargo test -p modern-chess-game` — Expected: all pass.
- [ ] **Step 7: Commit** `feat(games): Gliński's hexagonal chess`

### Task 5: The view says the board is hex

**Files:**
- Modify: `crates/session/src/view.rs`, `crates/session/src/session.rs`
- Modify: `apps/web/src/api/types.ts` (`View`), `apps/web/src/board.d.ts` if it declares the view
- Test: `crates/session/tests/view.rs`

**Interfaces:**
- Consumes: `Board.shape` (Task 1).
- Produces: `View { pub shape: BoardShapeView, pub layout: Option<HexLayoutView>, .. }` serialised as `"shape": "square" | "hex"` and, on hex boards only, `"layout": "rhombus" | "hexagon"` (omitted on square boards); TS `shape: "square" | "hex"; layout?: "rhombus" | "hexagon"`.

- [ ] **Step 1: Write the failing tests** in `crates/session/tests/view.rs`:
  - `a_square_board_says_square`: chess view JSON has `"shape":"square"` and no `layout` key.
  - `a_hexagon_board_says_hex_and_lists_its_corners`: Gliński view has `"shape":"hex"`, `"layout":"hexagon"`, `width == 11`, `height == 11`, `disabled.len() == 30`.
  - `a_rhombus_board_says_rhombus`: Hex view has `"layout":"rhombus"`, `disabled` empty.
- [ ] **Step 2: Run** `cargo test -p modern-chess-session --test view` — Expected: compile error / FAIL.
- [ ] **Step 3: Implement** the two fields from `game.board.shape`; add them to the TS `View` type.
- [ ] **Step 4: Run** `cargo test -p modern-chess-session` and `pnpm --filter web exec tsc --noEmit` (or the package's typecheck script) — Expected: pass.
- [ ] **Step 5: Commit** `feat(session): the view says when the board is hex`

### Task 6: The board element draws hexes

**Files:**
- Modify: `apps/docs/theme/board-model.js` (geometry, three colours, per-player zone tints), `apps/docs/theme/modern-chess-board.js` (hex layout in `renderGrid`), `apps/docs/theme/modern-chess-board.css`
- Modify: `apps/docs/src/11-design-your-own.md` (skin: third colour, per-player zone tints)
- Test: `apps/docs/theme/test/board-model.test.mjs`

**Interfaces:**
- Consumes: `view.shape`, `view.layout` (Task 5).
- Produces (in `board-model.js`):
  - `export function hexCentre(view, x, y, flipped) -> { left: number, top: number }` in units of the board's own box, 0–1 on each axis, top-left origin. Hexagon: `(1.5·q, √3·(r − q/2))` before scaling; rhombus: the same layout turned so the line from `(n−1, 0)` to `(0, n−1)` lies horizontal; flipped: `(q, r)` placed where `(w−1−q, h−1−r)` would be.
  - `export function hexColour(x, y) -> 0 | 1 | 2` = `(x + y) % 3`.
  - `normalizeSkin` keeps `squares.mid` when given; otherwise derives it as the mix of `light` and `dark`.
  - `zoneTint(skin, view, x, y)` accepts a zone entry that is a colour or `{ PLAYER: colour }`; for the per-player form it tints with the colour of the player whose zone cells contain `(x, y)`, the first such player in `view.players` order.

- [ ] **Step 1: Write the failing tests** in `board-model.test.mjs`:
  - `hex colours never repeat between neighbours`: for each of the six vectors `v`, `hexColour(5,5) !== hexColour(5+v[0], 5+v[1])`.
  - `the hexagon is upright and centred`: for a Gliński view (`width 11, height 11, layout "hexagon"`), `hexCentre(view, 5, 0)` is bottom-centre (`left ≈ 0.5`, `top` the largest of all cells) and `hexCentre(view, 5, 10)` top-centre.
  - `the rhombus lies wide`: for a Hex view (`11×11, layout "rhombus"`), `hexCentre` of `(10,0)` and `(0,10)` have equal `top` (≈ 0.5) and are the leftmost/rightmost cells.
  - `flipping turns the board round`: `hexCentre(view, x, y, true)` equals `hexCentre(view, w−1−x, h−1−y, false)` for a few cells.
  - `a mid colour is derived when a skin gives two`: `normalizeSkin({ squares: { light: "#ffffff", dark: "#000000" } }).squares.mid` is a colour between them.
  - `per-player zone tints`: with `skin.zones = { SIDE_A: { RED: "#f00", BLUE: "#00f" } }` and `view.zones.SIDE_A = { RED: [[0,3]], BLUE: [[3,0]] }`, `zoneTint` is `"#f00"` at `(0,3)` and `"#00f"` at `(3,0)`; a plain-colour zone still tints as before.
- [ ] **Step 2: Run** `node --test apps/docs/theme/test/` — Expected: the new tests FAIL.
- [ ] **Step 3: Implement** the helpers; in `renderGrid`, when `view.shape === "hex"`, build the same cell buttons (holes skipped) absolutely positioned at `hexCentre`, with a hex `clip-path` and the class `mcb-hex-0|1|2` from `hexColour` mapped to `light`/`mid`/`dark` (index 1 = mid, so Gliński's f6 is the mid tone); square boards are untouched. The board's box keeps the square board's width.
- [ ] **Step 4: Run** `node --test apps/docs/theme/test/` — Expected: PASS. Then `pnpm --filter web test` — Expected: PASS.
- [ ] **Step 5: See it.** Build the wasm bundle (`apps/docs/build.sh` or the web app's dev flow) and open Hex and Gliński in "Try it": the rhombus lies wide with RED's sides tinted red, the hexagon stands upright with three tones and f6 mid; clicking a cell on a flipped board plays that cell. Take a screenshot of each for the PR.
- [ ] **Step 6: Docs.** Document `squares.mid` and per-player zone tints in the spec guide's skin section.
- [ ] **Step 7: Commit** `feat(board): draw hex boards`
