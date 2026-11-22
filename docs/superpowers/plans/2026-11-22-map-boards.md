# Map boards (step 4b) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A spec can describe a map of points, paths and areas joined by labelled links (written
out, or generated from a hex layout), pieces move along the links, and the board element draws it,
without slowing grid games down.

**Architecture:** The engine's `Graph` gets a map form beside the grid one: grids keep their
implicit vector walk, a map stores one flat link table per label. `Labels` interns label names as
well as vectors; a move's `step` is a vector (grids) or a label name (maps), and a label step fans
out to every linked place. The session view lists map places with their kind, ends and outline;
the board element draws them in SVG.

**Tech Stack:** Rust (engine, session, wasm, game service), vanilla JS custom element
(`apps/docs/theme`), TypeScript/Preact web app, mdBook.

**Spec:** `docs/superpowers/specs/2026-11-22-map-boards-design.md`

## Global Constraints

- Every existing test keeps what it checks. Call sites may be respelled mechanically when a public
  type changes (as in steps 2 and 4a); assertions never change.
- Grid games stay within 5% of the speed baseline Task 1 records (median of 5 release runs per game).
- Map labels, exactly: `next`, `ends`, `paths`, `corners`, `areas`, `sides`.
- Place kinds, exactly: `point` (the default), `path`, `area`. Spec fields: `name`, `kind`, `at`, `ends`, `around`.
- Board shapes in the spec: `"map"` (with `places`, optional `links`) and `"hex-map"` (with `layout`, `size`, optional `disabled_positions`).
- Hex map names: tile `"q,r"`; side = its two tiles; corner = its three tiles; tiles sorted by (q, r) numerically and joined by `|`, e.g. `"1,2|2,2|2,3"`. Off-board tiles count, so q or r may be negative (`"-1,0"`).
- A place name is 1 to 64 bytes. A map has 1 to 10,000 places.
- On a map, `orthogonal` and `adjacent` are both `next`.
- Grid views keep sending whole-number `at` values; a map view's shape is `"map"`.
- Never split a node name into coordinates in the front end (4a's anti-goal).
- No new crate dependencies.
- Commits: conventional, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. pnpm for JS.

## Review Focus

1. A map spec with a typo in `ends`/`around` (unknown place, or a path or area named there) must be refused with an error naming the place, never panic or build a half map — Task 2 tests it.
2. A label step from a place with no link of that label gives no moves, and does not end the game wrongly or crash — Task 4 tests it.
3. A chess-like spec moved onto a map (vector steps, castling positions, a `direction`) must be refused when read, not load and silently never move — Task 4 tests each refusal.
4. A flipped map must still answer clicks with the right place name (the drawing turns, the names do not) — Task 6 tests it in the model and the browser check covers it.
5. Saving and loading a map game's state, and its spec's JSON round trip (the game service stores specs as JSON), must give the same game — Tasks 2 and 3 test the round trip, Task 4 the state.

---

### Task 1: The speed baseline

**Files:**
- Create: `crates/engine/examples/speed.rs`

**Interfaces:**
- Produces: `cargo run --release -p modern-chess-engine --example speed` printing one line per game: `<game> plies=<n> moves=<count> median_ms=<ms>`.

- [ ] **Step 1: Write `examples/speed.rs`.** It loads `specs/chess.json` (3 plies), `specs/shogi.json` (3 plies) and `specs/go.json` (9×9, 2 plies) through `GameSpec`/`Game::from_spec` and counts every legal move at every ply using only the public transitions: `CalculateMoves` on each of the current player's pieces and `ExecuteMove` on each destination of a clone, `CalculateDrops`/`ExecuteDrop` for every piece code the player can drop (Go's stones), taking the first option of a `Transform` phase. It runs each count 5 times and prints the median wall time. The move counts are a correctness check too: they must not change in any later task.
- [ ] **Step 2: Run it on this commit (before any 4b code)** and confirm the counts are stable across two runs. Expected: three lines; chess 3 plies prints `moves=` the same number both runs.
- [ ] **Step 3: Write the three lines into the report file** under "Baseline". The controller copies them into the ledger; later tasks compare against them.
- [ ] **Step 4: Commit** — `chore(engine): a timing run for legal move generation`.

### Task 2: Written maps in the spec

**Files:**
- Create: `crates/engine/src/specs/game/map.rs` (map spec types, compiling `places`/`links` into a checked `MapSpec`)
- Modify: `crates/engine/src/specs/game/board.rs` (`BoardShape::Map`, `BoardSpec.map`, raw parsing and serialising, `validate_size`)
- Modify: `crates/engine/src/specs/game/game.rs` (`GameSpecError` variants)
- Test: `crates/engine/src/tests/map_spec/mod.rs`

**Interfaces:**
- Produces:
  - `pub enum PlaceKind { Point, Path, Area }` (serde lowercase, default `Point`).
  - `pub struct PlaceSpec { pub name: String, pub kind: PlaceKind, pub at: [f64; 2], pub ends: Option<[String; 2]>, pub around: Vec<String> }` — after compiling, every place has `at` (points: as written; paths: midpoint of their ends; areas: average of their corners).
  - `pub struct MapSpec { pub places: Vec<PlaceSpec>, pub lines: Vec<[String; 2]>, pub links: BTreeMap<&'static str, Vec<Vec<usize>>> }` — `links[label][i]` = indices of the places place `i` links to under `label`, in board order, for each of the six labels; `lines` = the written `links` pairs.
  - `BoardShape::Map` and `BoardSpec { .., pub map: Option<Arc<MapSpec>> }`; a map board has empty `dimensions` and `disabled_positions`.
  - `pub fn MapSpec::index(&self, name: &str) -> Option<usize>`.
  - `GameSpecError::InvalidMap(String)` — every map refusal, with a readable message naming the place.

- [ ] **Step 1: Write the failing tests** in `tests/map_spec/mod.rs`, from a small written map (four points `a b c d` in a square, paths `ab bc cd da`, area `f` around `a b c d`, one written link `["a", "c"]`):
  - `places_keep_board_order_and_get_at`: names in written order; `ab`'s `at` is the midpoint of `a` and `b`; `f`'s is the average of its four corners.
  - `implied_links`: `next` from `a` = `[b, c, d]` (via `ab`, `da` and the written link); `next` from `ab` = `[bc, da]`; `ends` of `ab` = `[a, b]`; `paths` of `a` = `[ab, da]`; `corners` of `f` = `[a, b, c, d]`; `areas` of `a` = `[f]` and of `ab` = `[f]`; `sides` of `f` = `[ab, bc, cd, da]`. A second area `g` around `b e c` (add point `e`) is `next` to `f` (they share `b, c` in a row).
  - `refusals`: each of these gives `GameSpecError::InvalidMap` with the offending name in the message — duplicate name; empty name; a 65-byte name; `ends` naming an unknown place; `ends` naming a path; a path whose ends are equal; an area with two corners; `around` naming an area; a point without `at`; an empty `places`; a written link naming an unknown place; `disabled_positions` on a map.
  - `round_trip`: serialising the `BoardSpec` and reading it back gives the same places, kinds, `at` of points, ends, outlines and links.
- [ ] **Step 2: Run** `cargo test -p modern-chess-engine map_spec`. Expected: FAIL (module/types missing).
- [ ] **Step 3: Implement.** Raw form: `places` entries `{ name, kind?, at?, ends?, around? }`, `links: [[a, b], ...]`. Links are computed once, when the spec is read; board order is written order. Two areas are `next` when two consecutive outline points of one (wrapping round) are consecutive in the other, in either direction. Lists inside `links` are in board order without repeats. `validate_size` refuses a map with 0 or more than 10,000 places.
- [ ] **Step 4: Run** the map tests, then `cargo test -p modern-chess-engine`. Expected: all pass.
- [ ] **Step 5: Commit** — `feat(engine): maps of points, paths and areas in the spec`.

### Task 3: The hex map

**Files:**
- Create: `crates/engine/src/specs/game/hex_map.rs`
- Modify: `crates/engine/src/specs/game/board.rs` (`"hex-map"` shape)
- Test: `crates/engine/src/tests/hex_map/mod.rs`

**Interfaces:**
- Consumes: `MapSpec`, `PlaceSpec` (Task 2).
- Produces: `pub fn hex_map(layout: HexLayout, size: u8, disabled: &HashSet<Coordinates>) -> Result<MapSpec, String>`; `BoardSpec` from `{"shape": "hex-map", ...}` has `shape: BoardShape::Map`, `map: Some(..)`, and serialises back as `hex-map` with its layout, size and disabled positions (keep the source on `MapSpec`, e.g. `pub source: MapSource`).

- [ ] **Step 1: Write the failing tests:**
  - `catan_counts`: hexagon size 3 → 19 areas, 54 points, 72 paths; rhombus size 2 → 4 areas, 16 points, 19 paths.
  - `names`: the hexagon size 3 centre tile `"2,2"` exists as an area; its six corners are named from its neighbours in the cycle (0,1),(1,1),(1,0),(0,−1),(−1,−1),(−1,0): corner i = sorted {tile, n_i, n_(i+1)} joined by `|`, e.g. `"1,1|1,2|2,2"`; side `"2,2|2,3"` exists; a tile at the board's edge has a corner with a negative or off-board tile in its name.
  - `links_fit`: `corners` of every tile has 6 places; `areas` of a corner is 1 to 3 tiles; `next` from the centre tile has 6 tiles; `next` from a corner has 2 or 3 corners; `ends` of a side are 2 corners.
  - `at_matches_the_hex_board`: a tile's `at` is `(1.5·q, √3·(r − q/2))` (hexagon), turned by −120° on a rhombus (same as `hexPoint` in `apps/docs/theme/board-model.js`); a corner's `at` is the average of its three tiles' centres, a side's of its two.
  - `disabled_tiles`: hexagon size 3 with `"2,2"` disabled → 18 tiles, and the centre's six corners still exist (each touches other tiles).
  - `round_trip` of the hex-map board spec.
- [ ] **Step 2: Run** `cargo test -p modern-chess-engine hex_map`. Expected: FAIL.
- [ ] **Step 3: Implement** `hex_map`: tiles are the enabled cells of the same grid a hex board of that layout and size uses (reuse `hexagon_corners`); corners and sides exist when they touch at least one tile. Board order: tiles, then corners, then sides, each sorted by name. Build the `MapSpec` the same way Task 2 builds a written one, so links come from the same code (a side's ends are its two corners; a tile's outline is its six corners in cycle order).
- [ ] **Step 4: Run** the hex map tests, then the whole engine suite. Expected: all pass.
- [ ] **Step 5: Commit** — `feat(engine): generate a map of tiles, corners and sides from a hex layout`.

### Task 4: Playing on a map

**Files:**
- Create: `crates/engine/src/logic/game/board/map.rs` (the map form of the graph)
- Modify: `crates/engine/src/logic/game/board/graph.rs`, `board/mod.rs`
- Modify: `crates/engine/src/specs/game/piece/move.rs` (`StepSpec`), `player.rs` (optional `direction`, positions by name), `condition.rs` (POSITION by name), `game.rs` (map refusals; `validate_steps`/`validate_vector_sizes` skip label steps)
- Modify: `crates/engine/src/logic/blueprint/move_blueprint/{mod,build,generate}.rs`, `logic/game/mod.rs` (starting positions by name)
- Test: `crates/engine/src/tests/map_play/mod.rs`

**Interfaces:**
- Consumes: `MapSpec` (Tasks 2, 3).
- Produces:
  - `#[serde(untagged)] pub enum StepSpec { Vector(ExtendedPosition), Label(String) }` as `MoveSpec::step`.
  - `#[serde(untagged)] pub enum PositionSpec { Coordinates(Coordinates), Name(String) }` for starting positions and POSITION condition entries; both become `Position` names.
  - `Labels::named(&mut self, name: &str) -> Offset` (label and unit the same).
  - `Board::linked(&self, node: NodeId, label: LabelId) -> &[NodeId]` (empty on grids); `Board::places(&self) -> Option<&MapSpec>` (for the session); `orthogonal`/`adjacent` read `next` on maps; `coordinates` is `None` on maps.

- [ ] **Step 1: Write the failing tests** in `tests/map_play/mod.rs`, with specs built in the test from Task 2's square map:
  - `label_step_moves_to_every_linked_place`: a piece on `a` with `"step": "next"` and an `EMPTY` move action can go to `b`, `c`, `d`; with an ally on `c`, only `b`, `d`; executing to `b` moves it.
  - `no_link_no_moves`: a piece on an isolated point (a map with an extra point `z` and no links) has no moves, and the game is not over while another piece can move.
  - `paths_and_areas_hold_pieces`: a piece on path `ab` stepping `next` reaches `bc` and `da`; a piece on area `f` stepping `next` reaches `g`; a piece on `a` stepping `areas` reaches `f`.
  - `starting_positions_and_zones_by_name`: starting positions `["a"]` and a POSITION condition `{"WHITE": ["c"]}` load; a POSITION or starting name that is not a place is refused; a grid spec with starting position `"4,1"` loads the same as `[4, 1]`.
  - `go_liberties_and_connection_on_a_map`: a stone on `a` surrounded on `b`, `c`, `d` by enemy stones is captured (liberty capture uses `next`); a CONNECTION win condition with `"adjacent"` joins a chain along `next`.
  - `refusals_on_a_map`, each `GameSpecError` with a readable message: a vector step; a condition with `position`; a side effect with `from`/`to`/`target`; a player `direction`; an `ALLY_ON_FILE` drop restriction; a `repeat` on a label step; a coordinate starting position; an unknown label (`"step": "diagonal"`).
  - `refusals_on_a_grid`: a label step on chess's board.
  - `state_round_trip`: `save_state`/`restore_state` of a map game after one move gives the same pieces and turn.
- [ ] **Step 2: Run** `cargo test -p modern-chess-engine map_play`. Expected: FAIL.
- [ ] **Step 3: Implement.** The map graph keeps per label a flat CSR table (`offsets: Vec<u32>`, `targets: Vec<NodeId>`) built from `MapSpec.links`; tables exist for all six labels. A label step's blueprint marks itself as a label step; `calculate_moves` then judges each linked place exactly as one non-repeating step is judged today (conditions, actions, side effects, modifiers, post filters). The grid's `follow` and `calculate_moves` paths must not gain work beyond one check of the step's kind; keep `Graph`'s grid lookup as it is. On grids `direction` stays required (an absent one is refused as today's wrong-size matrix); on maps it must be absent. Existing respelling is mechanical (`step: vec![0, 1]` → `StepSpec::Vector(vec![0, 1])`).
- [ ] **Step 4: Run** the new tests, then `cargo test --workspace` and `make lint-panics`. Expected: all pass, lint clean.
- [ ] **Step 5: Speed check:** `cargo run --release -p modern-chess-engine --example speed`, twice. Expected: same move counts as the baseline; each median within 5% of the baseline in the ledger. Put the numbers in the report.
- [ ] **Step 6: Commit** — `feat(engine): pieces move along a map's links`.

### Task 5: The view lists a map

**Files:**
- Modify: `crates/session/src/view.rs`, `crates/session/src/session.rs`
- Modify: `apps/web/src/api/types.ts`, `apps/web/test/fixtures.ts` (only if types force it)
- Test: `crates/session/tests/map_view.rs`

**Interfaces:**
- Consumes: `Board::places()` (Task 4).
- Produces: `NodeView { name, at: [f64; 2], kind: Option<PlaceKind>, ends: Option<[String; 2]>, around: Option<Vec<String>> }` (the options skipped when `None`; grid nodes have none of them); `Shape::Map`; `View.lines: Vec<[String; 2]>` skipped when empty. TS: `BoardNode` gains optional `kind: "point" | "path" | "area"`, `ends`, `around`; `Shape` gains `"map"`; `View` gains optional `lines`.

- [ ] **Step 1: Write the failing tests** in `crates/session/tests/map_view.rs` (a session on Task 2's square map, as a JSON spec string): the view's `shape` is `"map"`, `layout` is absent; `nodes` lists every place in board order with `kind`, `at`, `ends` (`ab` → `["a","b"]`), `around` (`f`); `lines` is `[["a","c"]]`; selecting a piece on `a` gives destinations by name. And `grid_view_unchanged`: chess's view serialises `at` as `[4,0]` (whole numbers, no `.0`) and has no `kind`, `ends`, `around` or `lines` keys.
- [ ] **Step 2: Run** `cargo test -p modern-chess-session map_view`. Expected: FAIL.
- [ ] **Step 3: Implement.** For whole-number output on grids, serialise `at` through a helper that writes a value with no fraction as an integer. Update the TS types; `pnpm --dir apps/web typecheck`.
- [ ] **Step 4: Run** `cargo test --workspace`, `pnpm --dir apps/web test`, `pnpm --dir apps/web typecheck`. Expected: all pass.
- [ ] **Step 5: Commit** — `feat(session): the view lists a map's places, paths, areas and lines`.

### Task 6: The board element draws a map

**Files:**
- Modify: `apps/docs/theme/board-model.js`, `apps/docs/theme/modern-chess-board.js`, `apps/docs/theme/modern-chess-board.css`
- Test: `apps/docs/theme/test/board-model.test.mjs`

**Interfaces:**
- Consumes: the map view (Task 5).
- Produces: `mapGeometry(view)` → `{ minX, minY, maxX, maxY, unit }` (`unit`: the shortest distance between two points, 1 when fewer than two); `mapPlace(view, at, flipped, geometry)` → `{ left, top }` as shares of the board box (0 to 1, from the top left, y up in map units); `renderMap(view)` in the element, used when `view.shape === "map"`.

- [ ] **Step 1: Write the failing model tests:** bounds and `unit` of a map view; `mapPlace` of a corner point, unflipped and flipped (a flipped map turns 180° about its bounds' centre); a path's and an area's piece anchor is their `at`; a single-point map has `unit` 1 and does not divide by zero.
- [ ] **Step 2: Run** `node --test apps/docs/theme/test`. Expected: FAIL.
- [ ] **Step 3: Implement** the helpers and `renderMap`: an SVG with a padding of one `unit`, areas as polygons from `around` (zone tint as the fill when they have one), `lines` as thin lines, paths as thicker lines with a wider transparent stroke to click, points as dots; pieces drawn as on other boards, about `0.8 · unit` wide, at their place's `at`; selection and destination marks on any kind of place; every place answers `select`/`move`/`dropPiece` with its name, like a grid cell. Skin decorations are not drawn on maps.
- [ ] **Step 4: Run** `node --test apps/docs/theme/test`. Expected: all pass, the 34 existing ones included.
- [ ] **Step 5: Commit** — `feat(board): draw map boards`.

### Task 7: Pong hau k'i, and a map chapter

**Files:**
- Create: `crates/engine/specs/pong_hau_ki.json`, `crates/game/pages/pong-hau-ki.json`, `crates/engine/src/tests/pong_hau_ki/mod.rs`
- Create: `apps/docs/src/12-maps.md`, `apps/docs/src/boards/catan_sampler.json`, `apps/docs/src/boards/catan_sampler.skin.json`, a skin for Pong hau k'i in `src/boards/`
- Modify: `crates/game/tests/catalog.rs`, `apps/docs/src/SUMMARY.md`, `apps/docs/src/reference.md`, `apps/docs/build.sh` and `.gitignore` (copy `pong_hau_ki.json` like `go.json`), `crates/engine/src/tests/mod.rs`

**Interfaces:**
- Consumes: everything above.

- [ ] **Step 1: Write the failing engine test** `pong_hau_ki`: the spec loads; points `top-left (0,2)`, `top-right (2,2)`, `centre (1,1)`, `bottom-left (0,0)`, `bottom-right (2,0)`; links: the four corners to `centre`, `top-left`–`bottom-left`, `top-right`–`bottom-right`, `bottom-left`–`bottom-right` (no top side); BLACK on the top corners, WHITE on the bottom ones, WHITE first; WHITE's first moves are exactly to `centre` from either piece; a played-out line where a player cannot move ends with the other player winning; a threefold repetition is a draw. Add `"pong-hau-ki"` to the catalog test.
- [ ] **Step 2: Run** `cargo test -p modern-chess-engine pong_hau_ki` and `cargo test -p modern-chess-game catalog`. Expected: FAIL.
- [ ] **Step 3: Write the spec** (one piece `STONE` stepping `next` onto `EMPTY`, `stalemate_loses: true`, no leader, `draw_conditions.repetition_count: 3`) and the page (family `"Blocking games"`, tags `classic`, `two players`, a short description of the rules in the style of `hex.json`).
- [ ] **Step 4: Run** the two tests. Expected: PASS.
- [ ] **Step 5: Write the chapter** `12-maps.md`, "Maps": places and their kinds, links and their labels, `"step": "next"`, the refusals in one short list, then the Pong hau k'i board (playable), then the hex map in "Try it" (`catan_sampler.json`: hexagon size 3, one player, a `ROBBER` on `"2,2"`, a `SETTLEMENT` on one of its corners and a `ROAD` on one of its sides, each stepping `next` onto `EMPTY`). Add the map fields to `reference.md`. Add the chapter to `SUMMARY.md`.
- [ ] **Step 6: Build the book** (`apps/docs/build.sh`). Expected: builds; the smoke test (`node apps/docs/theme/test/smoke.mjs`) passes, extended to load the two new boards.
- [ ] **Step 7: Commit** — `feat(games): Pong hau k'i, and a chapter on maps`.

### Task 8: Check it end to end (controller)

- [ ] Speed run against the baseline, as in Task 4 Step 5. Expected: within 5%, same counts.
- [ ] `cargo test --workspace`, `make lint-panics`, `node --test apps/docs/theme/test`, `pnpm --dir apps/web test`, `pnpm --dir apps/web typecheck`. Expected: all pass.
- [ ] In a browser: Pong hau k'i played to a win in the book and through the game service (needs `make dev` and `pnpm dev`); the Catan sampler's robber, settlement and road each moved, flipped and not; chess, Go, Xiangqi, Hex, Gliński and Shogi look and play as on 4a.
