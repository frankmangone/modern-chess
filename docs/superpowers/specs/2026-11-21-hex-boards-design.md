# Step 3: hex boards

Step 3 of `docs/superpowers/specs/2026-11-21-engine-generalisation-design.md`. Decided with Frank
on 2026-11-21.

## Why

The hub is meant for games people propose (communities form around
user-proposed games), and many abstract games are played on hexes. Step 2 made the board a
graph inside the engine; this step lets a spec ask for a hex board, draws it, and proves it with
two classic games: Hex and Gliński's hexagonal chess.

## Goals

- A spec can describe a hex board in a few words, as a rhombus or a hexagon.
- A hex cell has six neighbours, so every rule that uses `orthogonal` or `adjacent` (liberties,
  explosions, ally counts, the new connection rule) works on hexes as it does on squares.
- The board element draws hexes, three-coloured, and flips for the second player.
- A new, general win condition: one chain of a player's pieces, joined through a neighbourhood,
  touches at least *k* of a list of zones.
- **Hex** (11×11 rhombus) is playable end to end and listed in a new family, "Connection games".
- **Gliński's hexagonal chess** (91-cell hexagon) is playable end to end and listed under
  "Chess and its variants". A stalemate is a win for the player who causes it, and its page
  says so.

## Non-goals

- Positions by name (moved to step 4: hex cells still fit `x, y`).
- Free-form map boards and pieces on paths or corners (step 4).
- The swap rule of Hex (the second player may take over the first stone): it needs a choice
  outside a normal move, which belongs with turn stages (step 5). Hex's page says the game is
  played without it for now.
- Y and Havannah. The win condition is general enough for Y and for Havannah's fork and bridge;
  Havannah's ring needs loop detection, which is not in this step.
- An unlisted-page flag: both games are listed, so it is not needed.

## Anti-goals

- A rule that only one game can use (a "Hex win" or "connect your sides" rule). The connection
  rule must stay general (Frank's direction: rules composed from elementary rules).
- Any change to how square boards behave, to stored commands (`{type, x, y}`) or to saved
  states.

## Constraints

- The engine crate denies `unwrap`, `expect` and indexing outside tests, and builds for wasm.
- A spec that uses the hex form is stored for good once a game uses it: the JSON below is the
  stored format.
- The board element (`apps/docs/theme/modern-chess-board.js`) is shared by the docs and the web
  app.

## Approaches

Picked by Frank on 2026-11-21:

1. **Hex board in a spec: a short form that compiles to the grid (picked)**, with
   `disabled_positions` still allowed on top. The alternative, a bare `"cells": "hex"` switch on
   today's `dimensions`, makes every hexagon game list its cut corners by hand (30 cells for
   Gliński).
2. **Drawing: the same cell buttons, laid out as hexes (picked).** Clicks, keyboard focus, screen
   reader labels, pieces, highlights and flipping keep working. A full SVG board was the
   alternative; it fits map boards in step 4 better, and duplicates all the interaction code now.
3. **One general connection rule (picked)** instead of a Hex-only rule.

## The design

### Hex coordinates

A hex cell is `(q, r)`, stored as `x, y` everywhere. The six neighbours are:

| direction | vector |
|---|---|
| up | `(0, 1)` |
| down | `(0, -1)` |
| up-right | `(1, 1)` |
| down-right | `(1, 0)` |
| up-left | `(-1, 0)` |
| down-left | `(-1, -1)` |

On a hexagon board, columns (`q`) are vertical files and cells are flat-topped: cell `(q, r)`
sits at `x = 1.5·q`, `y = √3·(r − q/2)` in units of the cell's radius (y up). A rhombus board is
drawn as a wide diamond, the traditional Hex look: the same layout turned so that the long
diagonal, from `(n−1, 0)` to `(0, n−1)`, lies horizontal (cells stay flat-topped, since a
hexagon turned by 60° looks the same). The colour of a cell is `(q + r) mod 3`: no two
neighbours share a colour, and a bishop's diagonal keeps it.

### The board spec

```json
"board": { "shape": "hex", "layout": "rhombus", "size": 11 }
"board": { "shape": "hex", "layout": "hexagon", "size": 6 }
```

- `shape` defaults to `"square"`, which is today's form (`dimensions`, `disabled_positions`).
- A hex `rhombus` of size *n* is the `n × n` grid.
- A hex `hexagon` of side *n* is the `(2n−1) × (2n−1)` grid with the cells where
  `|q − r| ≥ n` disabled: Gliński's board is `size: 6`, 91 cells.
- `disabled_positions` may punch further holes in either layout.
- The checker refuses `dimensions` together with `shape: "hex"`, a size of 0, and a size whose
  grid is over today's limit (10,000 cells).
- The board graph of a hex board uses the six vectors above for both `orthogonal` and
  `adjacent`. Steps, offsets, orientation matrices and line conditions work unchanged.
  `ALLY_ON_FILE` reads the file as `q`.
- The hexagon's cut corners are added to the board's disabled cells, so the session reports
  them in `view.disabled` like any hole.

### The view

The session's `View` gains `"shape": "square" | "hex"`. `width`, `height` and `disabled` keep
their meaning on the underlying grid. Nothing else changes, so the wasm wrapper and the game
service pass it through.

### Drawing hexes

- With `shape: "hex"`, the board element positions each cell button at its hex centre, by the
  layout's placement above, and gives it a hex outline (`clip-path`). The view therefore also
  says `"layout": "rhombus" | "hexagon"` on hex boards. Clicks, focus, labels, pieces, zone tints and destinations work
  as on squares.
- Flipped (the second player's seat), the board turns 180°: `(q, r)` is drawn at the place of
  `(width − 1 − q, height − 1 − r)`.
- A skin's `squares` gains an optional third colour: `{ "light", "mid", "dark" }`, indexed by
  `(q + r) mod 3`. A two-colour skin on a hex board falls back to three colours derived from its
  two. Which index is which colour is the builder's call, as long as Gliński's centre cell f6 is
  the mid tone.
- The board fits the space a square board would take: the wide diamond is scaled to the width.
- A skin's zone tint may be one colour (today) or one colour per player:
  `"zones": { "SIDE_A": { "RED": "#…", "BLUE": "#…" } }`, so a zone's cells are tinted by which
  player they belong to. A cell in two players' zones (Hex's corners) takes the first player's
  tint in turn order. Claude's call: it keeps drawing-only data out of the stored spec, and other
  games with per-player zones can use it.

### The connection win condition

```json
{ "type": "CONNECTION", "through": "adjacent", "zones": ["WEST", "EAST"], "at_least": 2,
  "pieces": ["STONE"] }
```

- After every move, the player who moved wins when one chain of their pieces, each joined to the
  next through `through` (`"orthogonal"` or `"adjacent"`), touches at least `at_least` of the
  `zones`.
- A zone is a declared `POSITION` condition, so each player has their own cells for it.
- `pieces` limits which pieces form the chain; empty means all. `at_least` defaults to the
  number of zones.
- The checker refuses unknown zones, an unknown neighbourhood, and an `at_least` of 0 or more
  than the number of zones.

### Hex

- Spec `crates/engine/specs/hex.json`: board `{ "shape": "hex", "layout": "rhombus", "size": 11 }`;
  players `RED` (moves first) and `BLUE`; one piece, `STONE`, with `unlimited_supply` and no
  moves (as in Go); no leader, no passes, no captures.
- RED joins the `q = 0` and `q = 10` sides, BLUE the `r = 0` and `r = 10` sides. As zones:
  `SIDE_A` and `SIDE_B`, each with RED's and BLUE's cells. Win condition: `CONNECTION` through
  `adjacent`, zones `SIDE_A`, `SIDE_B`.
- A draw cannot happen: the board fills only after someone has connected.
- Drawn as a wide diamond: RED's sides run along the top-right and bottom-left edges, BLUE's
  along the other two.
- Page `crates/game/pages/hex.json`: family "Connection games", a red and blue skin with each
  player's sides tinted in their colour, and a how-to-play section that says the swap rule is
  not played yet.

### Gliński's hexagonal chess

- Spec `crates/engine/specs/glinski.json`: board `{ "shape": "hex", "layout": "hexagon",
  "size": 6 }`; players `WHITE` and `BLACK`, Black turned 180° (`[[-1, 0], [0, -1]]`); leader
  `KING`; `stalemate_loses: true`; no castling.
- Draws: threefold repetition and the fifty-move rule (pawn moves and captures reset it), as in
  chess. Insufficient material is only king against king: chess's other lists (king and bishop or
  knight against king) would end games a stalemate could still win, since a stalemate is a win
  here. Claude's call.
- Notation: files a–l without j are `q = 0…10`; rank *n* of file `q` is
  `r = max(0, q − 5) + n − 1`.
- Starting position, from the standard Gliński setup. **This has to be checked against the
  setup diagram before the spec is written.** The sources read so far confirm Qe1, Qe10, Bf3,
  pawn f5, and Black's pawns on rank 7.
  - White: K g1, Q e1, R c1 i1, N d1 h1, B f1 f2 f3, P b1 c2 d3 e4 f5 g4 h3 i2 k1.
  - Black: K g10, Q e10, R c8 i8, N d9 h9, B f9 f10 f11, P b7 c7 d7 e7 f7 g7 h7 i7 k7.
- Moves, as White's vectors:
  - Rook: slides along the six neighbour vectors.
  - Bishop: slides along the six diagonals `(1, 2)`, `(2, 1)`, `(1, −1)`, `(−1, −2)`,
    `(−2, −1)`, `(−1, 1)`.
  - Queen: both. King: one step along any of the twelve.
  - Knight: two steps along a neighbour vector, then one at 60°. That gives `(1, 3)`, `(−1, 2)`,
    `(3, 2)`, `(2, 3)`, `(2, −1)`, `(3, 1)`, `(−1, −3)`, `(1, −2)`, `(−3, −2)`, `(−2, −3)`,
    `(−2, 1)`, `(−3, −1)`.
  - Pawn: moves `(0, 1)` to an empty cell. It captures `(−1, 0)` and `(1, 1)`, including en
    passant as in chess.
  - Pawn double step `(0, 2)` through an empty cell, from any cell where a pawn of its own colour
    starts. A pawn that captures onto such a cell gets the double step again.
  - A pawn promotes on the last cell of its file (11 cells per side) to a queen, rook, bishop or
    knight.
- Check and checkmate as in chess. A player with no legal move loses (`stalemate_loses`), which
  scores the stalemate as a win for the other side. The page says that Gliński scored it ¾–¼
  and the hub has no fractional results.
- Page `crates/game/pages/glinski.json`: family "Chess and its variants", the walnut chess
  pieces, three wood tones.

### Docs

`apps/docs` (the spec guide) gains the hex board form, the `CONNECTION` win condition and the
three-colour skin.

## Decided after review

A builder read this document before planning and raised four points:

- This document replaces the route doc's step 3 where they differ: Frank chose classic Hex,
  listed, instead of an original unlisted game (2026-11-21). The route doc is updated to match.
- Gliński's draw rules: see that section (Claude's call).
- Hex's side tints: per-player zone tints in skins (Claude's call).
- Hex is drawn as a wide diamond (Frank's call, 2026-11-21).

## Proof

- Hex and Gliński's chess are played end to end in "Try it" and in a match.
- Engine tests cover:
  - the hex graph (neighbours, the hexagon's 91 cells, the rhombus edges);
  - the connection rule (a winning chain, a chain one cell short, a chain through the wrong
    neighbourhood, `at_least`);
  - Gliński piece moves from the centre, the pawn double step from a borrowed starting cell,
    promotion on a bent edge, and stalemate as a win.
- A session test shows a hex view.

## Left to the builder

- The Rust shape of `BoardSpec` for the two forms (an enum or optional fields), as long as the
  JSON above parses and today's square specs parse unchanged.
- How the connection rule finds chains (flood fill from the zone cells is enough at these
  sizes).
- Exact hex geometry in CSS, and how the third skin colour is derived from two.
- Page copy and skin colours.
- How en passant and the double step are expressed with today's conditions and state flags
  (chess's own spec is the model).
