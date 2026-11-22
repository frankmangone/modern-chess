# Positions by Name Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every layer outside the engine addresses a board position by its node name, and the view lists the board's nodes with where to draw them.

**Architecture:** The session takes names and builds a view whose `nodes` list replaces `width`/`height`/`disabled`; every other position in the view is a name. The game service's commands carry `position`, and a migration renames logged commands. The board element draws from `nodes`, finds everything by name, and answers clicks with names; the web app passes names through.

**Tech Stack:** Rust (`modern-chess-session`, `modern-chess-wasm`, `modern-chess-game` with sqlx/Postgres), vanilla JS web component (`apps/docs/theme`, `node --test`), Solid + TypeScript (`apps/web`, vitest), pnpm.

**Spec:** `docs/superpowers/specs/2026-11-21-position-names-design.md`

## Global Constraints

- Names only: no layer outside the engine accepts or sends `x, y` for a position. Grid cells keep the engine's names (`"4,2"`).
- The front end never splits a name into coordinates; it knows a node by `name` and draws it at `at`.
- Every game in `crates/engine/specs/` plays and looks exactly as today. Existing tests keep what they check; call sites and expected values are respelled mechanically (`select(4, 1)` → `select("4,1")`, `[4, 3]` → `"4,3"`, `disabled == [[1, 2]]` → "`"1,2"` is not a node").
- View: `nodes: [{ "name", "at": [x, y] }]` in board order, a disabled cell not listed; `width`, `height`, `disabled` removed; `shape`, `layout` and every non-position field unchanged.
- Session refusal of a name that is not a node: `Err("that square is not on the board")`, game unchanged.
- Commands: `{"type": "select" | "move", "position": "4,2"}`, `{"type": "drop", "code", "position"}`. A position over 64 bytes is answered 400 and not logged; a name that is not a node is answered 422 `"that is not a legal move"` and logged as rejected.
- Commits: conventional, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Use pnpm. Never touch the process on port 3001. No pushes.

## Review Focus

- A flipped board with holes: a click must land on the node drawn under it — Task 3, `a flipped grid is turned half way round`.
- Odd names (`""`, `"4, 1"`, `"e4"`, `"4,1,0"`, `"9,9"`) must be refused without changing the game — Task 1, `an_unknown_name_is_refused_and_leaves_the_game_as_it_was`.
- The old `x, y` command and a 65-byte position must be refused before anything is logged, while a 64-byte one is merely an illegal move — Task 2, malformed-command and boundary tests.
- A name that is not a node, sent through the game service, must be logged as rejected and leave the match's revision unchanged — Task 2, `a_square_not_on_the_board_is_refused_and_logged`.
- The hex outline computed from nodes must equal today's (hexagon 11: aspect 0.892269; rhombus 11: aspect 1.679564) — Task 3, `hex geometry from nodes matches the layout`.

---

### Task 1: The session and the wasm wrapper speak names

**Files:**
- Modify: `crates/session/src/view.rs`, `crates/session/src/session.rs`, `crates/wasm/src/lib.rs`
- Modify (temporary call-site shim): `crates/game/src/http/play.rs:152-155`
- Test (respell + new): `crates/session/tests/*.rs`, `crates/wasm/tests/identical.rs`, `crates/game/tests/play.rs` (only where it reads the view), `apps/docs/theme/test/smoke.mjs`
- Docs: `apps/docs/README.md` (the Session row)

**Interfaces:**
- Produces (`view.rs`): `pub struct NodeView { pub name: String, pub at: [u8; 2] }`; `PieceView { pub position: String, pub code: String, pub player: String }`; `View` gains `pub nodes: Vec<NodeView>` and loses `width`, `height`, `disabled`; `selected: Option<String>`, `destinations: Vec<String>`, `zones: BTreeMap<String, BTreeMap<String, Vec<String>>>`, `drops: BTreeMap<String, Vec<String>>`. The `Square` type is removed. Every list of names is in board order (`Position`'s `Ord`), pieces included.
- Produces (`session.rs`): `select(&mut self, position: &str)`, `make_move(&mut self, position: &str)`, `drop_piece(&mut self, code: &str, position: &str)`, all `-> Result<(), String>`.
- Produces (`crates/wasm`): `Board::select(&mut self, position: &str)`, `make_move(&mut self, position: &str)`, `drop_piece(&mut self, code: &str, position: &str)`.
- Nodes come from `game.board.all_positions()` with `game.board.coordinates(&p)` as `at` (the session already refuses boards that are not 2D). Zones pass the condition's names through, keeping only names that are nodes.

- [ ] **Step 1: Write the failing tests** in `crates/session/tests/view.rs` (and a new `crates/session/tests/names.rs` for refusals):

```rust
#[test]
fn the_view_lists_every_node_with_where_to_draw_it() {
    let view = Session::new(CHESS).unwrap().view();
    assert_eq!(view.nodes.len(), 64);
    assert_eq!(view.nodes[0], NodeView { name: "0,0".into(), at: [0, 0] });
    assert!(view.nodes.contains(&NodeView { name: "4,2".into(), at: [4, 2] }));
}

#[test]
fn a_disabled_cell_is_not_a_node() { // the 3×5 spec with a hole at [1, 2]
    let view = /* existing view_reports_disabled_squares spec */;
    assert_eq!(view.nodes.len(), 14);
    assert!(!view.nodes.iter().any(|n| n.name == "1,2"));
}
// a_hexagon_board_says_hex_and_lists_its_corners: nodes.len() == 91 (was disabled.len() == 30)
// a_rhombus_board_says_rhombus: nodes.len() == 121 (was disabled empty)

#[test]
fn pieces_and_highlights_are_named() {
    let mut s = Session::new(CHESS).unwrap();
    s.select("4,1").unwrap();
    let view = s.view();
    assert_eq!(view.selected.as_deref(), Some("4,1"));
    assert_eq!(view.destinations, vec!["4,2".to_string(), "4,3".to_string()]);
    assert!(view.pieces.contains(&PieceView { position: "4,0".into(), code: "KING".into(), player: "WHITE".into() }));
    assert!(view.zones["REACH_END"]["WHITE"].contains(&"0,7".to_string()));
}

#[test]
fn an_unknown_name_is_refused_and_leaves_the_game_as_it_was() {
    let mut s = Session::new(CHESS).unwrap();
    let before = s.view();
    for name in ["", "4, 1", "e4", "4,1,0", "9,9"] {
        assert_eq!(s.select(name), Err("that square is not on the board".to_string()));
        assert_eq!(s.make_move(name), Err("that square is not on the board".to_string()));
        assert_eq!(s.drop_piece("PAWN", name), Err("that square is not on the board".to_string()));
        assert_eq!(s.view(), before);
    }
}
```

Respell every other session, wasm and game-test call site and view read as the Global Constraints say. In `crates/game/tests/play.rs` only the view reads change here (`body["view"]["selected"] == json!("4,1")`, `p["position"] == "0,3"`); its commands still send `x, y` until Task 2.

- [ ] **Step 2: Run** `cargo test -p modern-chess-session 2>&1 | tail -20`. Expected: compile errors naming `nodes`, `NodeView` and the `&str` arguments.

- [ ] **Step 3: Implement** the Interfaces above in `view.rs`, `session.rs` and `crates/wasm/src/lib.rs`. Check the name is a node (`game.board.node(&Position::from(position))`) before anything else in each of the three calls. In `crates/game/src/http/play.rs`, shim the three arms with `&format!("{x},{y}")`; Task 2 removes the shim.

- [ ] **Step 4: Run** `cargo test -p modern-chess-session -p modern-chess-wasm -p modern-chess-engine && cargo test -p modern-chess-game`. Expected: all pass. Then `apps/docs/build-engine.sh && node apps/docs/theme/test/smoke.mjs` with `smoke.mjs` respelled (`board.select("4,1")`, `game.drop_piece("BISHOP", name)`, …). Expected: exit 0.

- [ ] **Step 5: Commit** `feat(session): positions by name, and a view that lists the board's nodes`.

### Task 2: Game service commands carry a position

**Files:**
- Modify: `crates/game/src/domain/command.rs`, `crates/game/src/http/play.rs`
- Create: `crates/game/migrations/0004_position_names.up.sql`, `crates/game/migrations/0004_position_names.down.sql`
- Test: `crates/game/tests/page_and_commands.rs`, `crates/game/tests/play.rs`, `crates/game/tests/store.rs`
- Docs: design doc's "refused at the API (422)" becomes 400 (malformed commands already answer 400 and are never logged)

**Interfaces:**
- Consumes: Task 1's session calls.
- Produces: `CommandKind::Select { position: String }`, `Move { position: String }`, `Drop { code: String, position: String }`; `CommandKind::position(&self) -> Option<&str>`; `pub const MAX_POSITION_BYTES: usize = 64;` in `command.rs`.

- [ ] **Step 1: Write the failing tests.**
  - `page_and_commands.rs`: `CommandKind::Move { position: "4,3".into() }` serializes to `json!({ "type": "move", "position": "4,3" })`; likewise select and drop.
  - `play.rs`: helpers `select(name: &str)`, `go(name: &str)`, `drop_piece(code, name)` send `position`. The malformed list replaces the `x: 300` case with `{ "type": "select", "x": 4, "y": 1 }` and `{ "type": "select", "position": "a".repeat(65) }`; all answer 400 and the log stays empty.
  - `play.rs`, new: `a_square_not_on_the_board_is_refused_and_logged` — `select("9,9")` and `select(&"a".repeat(64))` each answer 422 `{"error": "that is not a legal move"}`; the log holds two rejected commands; the match's revision is unchanged.
  - `store.rs`, new: `logged_grid_commands_are_renamed_by_the_migration` — on a migrated database with a stored lobby, insert raw rows with kinds `{"type":"move","x":4,"y":3}` and `{"type":"drop","code":"PAWN","x":2,"y":2}`, run `include_str!("../migrations/0004_position_names.up.sql")`, and `CommandRepo::list` returns `Move { position: "4,3" }` and `Drop { code: "PAWN", position: "2,2" }`.

- [ ] **Step 2: Run** `cargo test -p modern-chess-game 2>&1 | tail -20`. Expected: compile errors on `position`.

- [ ] **Step 3: Implement.** Change `CommandKind` and remove Task 1's shim. In the handler, refuse a position longer than `MAX_POSITION_BYTES` with `ApiError::new(StatusCode::BAD_REQUEST, "that position is too long")` before the revision check and before anything is logged. The up migration:

```sql
-- Commands logged before positions were named carry x and y; give them the grid square's name.
UPDATE game_commands
SET kind = (kind - 'x' - 'y') || jsonb_build_object('position', (kind->>'x') || ',' || (kind->>'y'))
WHERE kind ? 'x' AND kind ? 'y';
```

The down migration turns `position` values matching `^[0-9]+,[0-9]+$` back into integer `x` and `y`, and leaves any other name as it is.

- [ ] **Step 4: Run** `cargo test --workspace 2>&1 | grep -E "^test result|FAILED"`. Expected: every line `ok`, no `FAILED`. Then `make lint-panics`. Expected: exit 0.

- [ ] **Step 5: Commit** `feat(game): commands name the square they are about`.

### Task 3: The board element and the web app draw from nodes

**Files:**
- Modify: `apps/docs/theme/board-model.js`, `apps/docs/theme/modern-chess-board.js`
- Modify: `apps/web/src/api/types.ts`, `apps/web/src/components/remote-board.tsx`, `apps/web/src/test/fixtures.ts`
- Test: `apps/docs/theme/test/board-model.test.mjs`, `apps/web/src/components/board-local.test.ts`, `apps/web/src/components/board-remote.test.ts`, `apps/web/src/pages/play.test.tsx`

**Interfaces:**
- Consumes: Task 1's view; Task 2's command JSON.
- Produces (`types.ts`): `interface BoardNode { name: string; at: [number, number] }`; `BoardView.nodes: BoardNode[]`, `pieces: { position: string; code: string; player: string }[]`, `selected: string | null`, `destinations: string[]`, `zones: Record<string, Record<string, string[]>>`, `drops: Record<string, string[]>`; no `width`, `height`, `disabled`, `Square`. `Command` select/move `{ position: string }`, drop `{ code: string; position: string }`.
- Produces (`board-model.js`): `gridBounds(view) -> { minX, minY, maxX, maxY }` over the nodes' `at`; `gridRows(view, flipped) -> (BoardNode | null)[][]`, rows top first in drawing order, `null` where no node is drawn; `zoneTint(skin, view, name)`; `hexGeometry(view)` with the outline taken from the nodes; `hexCentre(view, at, flipped = false, box = hexGeometry(view))`, flipping within `gridBounds`; `hexColour(x, y)` unchanged (called with `at`).
- Produces (element): events `select` / `move` with detail `{ position }`, `dropPiece` with `{ code, position }`; local steps `["select", name]`, `["make_move", name]`, `["drop_piece", code, name]`; cell labels `` `${player} ${code} at ${name}` `` and `` `empty square ${name}` `` (`empty cell` on hexes).

- [ ] **Step 1: Write the failing tests** in `board-model.test.mjs`, with a helper `grid(width, height, holes = [])` that builds `nodes` named `"x,y"` at `[x, y]`:

```js
test("grid rows put the first row at the bottom and leave holes empty", () => {
  const view = { nodes: grid(3, 2, ["1,1"]) };
  assert.deepEqual(gridRows(view, false).map((r) => r.map((n) => n?.name ?? null)),
    [["0,1", null, "2,1"], ["0,0", "1,0", "2,0"]]);
});
test("a flipped grid is turned half way round", () => {
  const view = { nodes: grid(3, 2, ["1,1"]) };
  assert.deepEqual(gridRows(view, true).map((r) => r.map((n) => n?.name ?? null)),
    [["2,0", "1,0", "0,0"], ["2,1", null, "0,1"]]);
});
test("bounds come from the nodes", () => {
  const view = { nodes: grid(4, 2).filter((n) => n.at[0] > 0) };
  assert.deepEqual(gridBounds(view), { minX: 1, minY: 0, maxX: 3, maxY: 1 });
});
test("hex geometry from nodes matches the layout", () => {
  // hexagon side 6: the 11×11 grid without |x − y| ≥ 6; rhombus: all 121.
  assert.equal(+hexGeometry(hexView("hexagon", 6)).aspect.toFixed(6), 0.892269);
  assert.equal(+hexGeometry(hexView("rhombus", 11)).aspect.toFixed(6), 1.679564);
});
```

Respell the existing zone-tint and hex tests to names and nodes, keeping their expectations. Respell the web tests' fixtures and event expectations (`{ position: "4,1" }`).

- [ ] **Step 2: Run** `node --test apps/docs/theme/test/board-model.test.mjs`. Expected: failures naming `gridRows` / `gridBounds`.

- [ ] **Step 3: Implement.** `renderGrid` walks `gridRows`, drawing a hole for `null`; `renderHexes` walks `view.nodes`; `renderLines` sizes from `gridBounds`; pieces, highlights, drops and tints are looked up by name (`Map`/`Set` keyed by name); `onCell(node)` emits names. Then the web app's types, `remote-board.tsx` (events pass `position` through) and fixtures.

- [ ] **Step 4: Run** `node --test apps/docs/theme/test/board-model.test.mjs && (cd apps/web && pnpm test && pnpm typecheck)`. Expected: all pass, typecheck exit 0. `grep -rnE 'split\(","\)' apps/docs/theme/*.js apps/web/src` prints nothing: no name is split.

- [ ] **Step 5: Commit** `feat(board): draw the board from its nodes and answer with names`.

### Task 4: Every board style, checked in a browser

**Files:** none expected; a fix found here goes into the task that owns the code, test first.

- [ ] **Step 1:** `apps/docs/build-engine.sh`, then serve a scratch page (in the session scratchpad, not the repo) that loads `apps/docs/theme` and shows local boards for chess, Go, Xiangqi, 4-player chess, Hex, Gliński and Shogi, plus a 3×3 square test spec with holes at `[1, 1]` and `[2, 2]`, on a port other than 3001 and 3050.
- [ ] **Step 2:** On each board: it looks as on `main` (compare screenshots), a move by clicking works, a flipped board's click lands on the cell under it, and the holed board draws its holes. Shogi: drop a captured piece. Go: place a stone.
- [ ] **Step 3:** A match through the game service: if the user's servers (3001, 3050) are running, play a few moves in the web app; if they are not, ask the user to start them. Do not start or stop anything on 3001.
- [ ] **Step 4:** Ledger the result. No commit unless a fix was needed.
