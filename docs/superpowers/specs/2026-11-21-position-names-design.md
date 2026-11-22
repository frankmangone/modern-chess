# Positions by name outside the engine (step 4a)

## Why

Step 4 of the engine route (`2026-11-21-engine-generalisation-design.md`) brings map boards: nodes
and edges written out in a spec, drawn as dots and lines. A map node has a name, not a column and
a row. The engine has named its positions since step 2 (a grid square is `"4,2"`), but every layer
above it still speaks `x, y`: the session, the wasm wrapper, the game service's commands, the web
app and the board element. This step moves all of them to names, so that maps (step 4b) plug in
without another change to the way positions travel.

Frank split step 4 into three pull requests on 2026-11-21, in this order: positions by name (this
one), map boards with pieces on paths and corners (4b), places that hold many pieces (4c).

## Goals

- Every layer outside the engine addresses a position by its node name, and only by its name.
- The view lists the board's nodes, each with where to draw it. The board element draws whatever
  nodes it is given and answers with the name of the one clicked.
- Every game in `crates/engine/specs/` and every board in the docs book plays and looks exactly as
  it does today.

## Non-goals

- **Keeping `x, y` working.** The route doc kept `x, y` accepted for old clients and stored
  commands. Frank chose the cheaper path on 2026-11-21: names only. Nothing is in production, the
  command log is never replayed, and the only clients (the web app and the docs book) change in
  this pull request. Saved sessions from before this branch do not have to survive it.
- Map boards, edges in the view, and readable labels such as `e4` (step 4b).
- Places that hold many pieces (step 4c).
- Drawing boards that are not 2D. The view shows 2D grids only, as today; 3D boards live in engine
  tests and the terminal app.
- The spec format. Specs keep writing positions and vectors as coordinates.

## Anti-goals

- A front end that turns names back into coordinates (splitting `"4,2"`). The board element knows
  a node only by its name and its drawing position.
- Two forms of position in any layer.

## Approaches

- **The view lists every node (chosen by Frank).** A `nodes` list gives each node's name and its
  drawing position; every other position in the view is a name. The board element draws from the
  list. Costs a bigger view (361 small entries for Go) and a change to the board element's drawing
  code now, not only to its inputs. Map boards then only add their own drawing positions and edges.
- **Names everywhere, grid implied.** The view keeps `width` and `height`; positions become names.
  Smaller now, but the board element must build `"x,y"` names to match cells to pieces, which leaks
  the engine's naming into the front end and has to be undone in 4b.
- **Accept both names and `x, y`.** Ruled out by Frank (above).

## The design

### Session (`crates/session`)

- `select(position: &str)`, `make_move(position: &str)` and `drop_piece(code: &str, position: &str)`
  take a node name.
- A name that is not a node of the board is refused before the engine is asked, with a readable
  error, and the game is left as it was (as with any refused call today).

### The view

| Today | After |
|---|---|
| `width`, `height`, `disabled: [[x, y]]` | `nodes: [{ "name": "4,2", "at": [4, 2] }]`; a disabled cell is simply not listed |
| `pieces: [{ x, y, code, player }]` | `pieces: [{ position, code, player }]` |
| `selected: [x, y] \| null` | `selected: name \| null` |
| `destinations: [[x, y]]` | `destinations: [name]` |
| `zones: zone → player → [[x, y]]` | `zones: zone → player → [name]` |
| `drops: code → [[x, y]]` | `drops: code → [name]` |

- `at` is the node's drawing position. On a square board it is its column and row; on a hex board
  it is the hex coordinates the board element already places with. Both are whole numbers today;
  step 4b decides the units of a map's drawing positions.
- `shape` and `layout` stay. Everything else in the view (hands, captured, material, supply,
  passes, score, status, promotion, turn) is unchanged.

### Wasm wrapper (`crates/wasm`)

The same three calls take a name, as in the session.

### Game service (`crates/game`)

- `CommandKind::Select { position }`, `Move { position }` and `Drop { code, position }`, with
  `position: String`. The JSON is `{"type": "move", "position": "4,2"}`.
- A name that is not a node is refused and logged like any illegal move today.
- A position longer than 64 bytes is refused at the API (422), before the session sees it or
  the command is logged. Claude's call: names are short, and the log should not store whatever a
  client sends.
- Claude's call: a migration rewrites the `x, y` commands already in the log into names
  (`{"x": 4, "y": 2}` becomes `{"position": "4,2"}`), so `CommandKind` can still read every row.
  The saved game states need nothing: they already hold names. It has a `.down.sql` that turns
  grid names back into `x, y`, like the existing migrations.

### Web app (`apps/web`)

- `api/types.ts`: a position is a string; the view and command types follow the table above.
- `remote-board.tsx` passes the board element's events through as `{ type, position }` (and
  `code` for drops).

### Board element (`apps/docs/theme`)

- Draws from `view.nodes`. A square board's grid is sized from the nodes' drawing positions, with a
  hole wherever no node sits (Claude's call: an edge row or column made only of holes is not
  drawn; no game has one); hex cells are placed from `at` as today; intersection lines (Go,
  Xiangqi) use the same bounds. Skin decorations (star points, the river, palace diagonals,
  labels) stay in drawing positions, as today.
- Pieces, highlights and zone tints are found by node name.
- A click emits the node's name: `select`, `move` and `dropPiece` events carry `position`, and the
  local engine calls (and the steps replayed after an engine crash) pass names.
- Cell labels for screen readers name the node (`"WHITE KING at 4,0"`), which reads as today on a
  grid.

### Docs

- `apps/docs/README.md` and the book's smoke test follow the new calls.
- The route doc records the split of step 4, the names-only call, and Frank's note for 4c: a place
  has a capacity, 1 unless the spec says otherwise.

## Proving it

- Every existing test keeps what it checks. Call sites and expected values are respelled
  mechanically (`select(4, 1)` becomes `select("4,1")`, `[4, 3]` becomes `"4,3"`), as in step 2.
- New tests: the view lists exactly the board's nodes (none for a disabled cell, on Gliński and on
  the small square test board with a hole in `crates/session/tests/view.rs`); an unknown name is refused and leaves the game unchanged, in the session and
  through the game service; an over-long position is refused at the API; the migration turns
  logged `x, y` commands into names.
- The board element's model tests cover drawing bounds and lookups by name.
- In a browser, one game of each board style looks and plays as before: chess (squares), Go and
  Xiangqi (intersections), 4-player chess (four seats), Hex and Gliński (hexes, and holes),
  Shogi (hands and drops), and a game played through the game service. No game in the catalogue
  has a square board with holes, so a scratch page draws a small square test board with holes
  (Claude's call).

## Left to the builder

- The order of the nodes in the view.
- Names of new types and helpers, and where they live.
- The error texts, kept in the style of today's.
- How the board element computes its bounds and indexes nodes, as long as no name is ever split
  into coordinates.
