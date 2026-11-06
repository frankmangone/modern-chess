# Playable boards in the docs: design

Date: 2026-11-04. Status: draft for review.

## Intent

The documentation book (`apps/docs`, mdBook) teaches people to write game specs, one example
per chapter. Today a reader has to imagine what a spec produces. This project puts a **live,
playable board** next to each example: the page loads the spec, runs the real engine in the
browser, and the reader clicks a piece to see its legal moves and clicks a square to play.

Why this, and why now:

- Readers learn faster when they can try the rule they just read. Every chapter already asks
  "run it and see"; the board removes the need to install anything.
- It proves the engine is faithful: the board shows the engine's answer, never a
  re-implementation of the rules in JavaScript.
- It is the first step toward the platform. The engine-in-the-browser wrapper and the board
  component are the pieces the platform's game page will need.

## Goals

1. A reader can open a chapter, see the example's board, click a piece, see its legal
   destinations highlighted, click one, and watch the move happen.
2. Turns switch automatically. After a move the other player is up, and the board says whose
   turn it is. Both sides are played from the same board (hot-seat). There is no opponent AI.
3. The board shows what the engine decides, including promotion choices, forced continuation
   of jumps, and the end of the game (winner or draw).
4. Looks are **decoupled from the spec**. A spec describes rules only. A separate **skin** file
   says how pieces and squares look.
5. A game with no skin still renders and is playable, using a plain fallback look.
6. The same board component can be reused by the future platform, not only the docs.

## Non-goals (for the first version)

- Dropping pieces from the hand (chapter 7's Shogi). Boards for hand games show the board
  only and say drops are not supported yet. Adding drops is a follow-up.
- Undo, move list, animations, drag-and-drop, sound, an AI opponent, online play.
- A skin editor, or art for every shipped game. A couple of example skins are enough.
- Changing the rules of the engine or the spec format. Engine changes are limited to the
  string parser described below and small additive accessors (see "Decisions already made").

## Anti-goals (this fails even if it technically works)

- Rules logic written in JavaScript. If the board ever decides legality itself, it will
  drift from the engine and teach wrong things.
- Any presentation field creeping into the spec (symbols, colors, images).
- A docs build that only works on the author's machine. Building the book must be one
  documented command, with the toolchain prerequisites listed.
- A book page that is blank or broken when WebAssembly fails to load. It must degrade to a
  readable message and keep the rest of the chapter intact.

## Design

Three pieces, each usable alone.

### 1. The engine in the browser: `crates/wasm`

A new workspace crate that wraps the engine with `wasm-bindgen` and compiles to
WebAssembly. The engine's dependencies are all pure Rust (serde, serde_json, thiserror,
parity-scale-codec), so the engine itself should compile unchanged.

One engine change: `parse_spec` takes a file path today. Add a sibling that takes the JSON
text (for example `parse_spec_str`), sharing the same validation. The path version calls it.

The wrapper exposes a single `Session` object. Its API is deliberately small and
**does not expose the engine's internal state type**:

| Call | Does |
|------|------|
| `new Session(specJson)` | Parses and validates the spec. Throws a readable error naming the problem (the same messages the chapters document). |
| `session.view()` | Returns a JSON snapshot for drawing (below). |
| `session.select(pos)` | Selects the piece at `pos` and computes its legal destinations (engine: `CalculateMoves`). |
| `session.move(pos)` | Plays the selected piece to `pos` (engine: `ExecuteMove`). |
| `session.promote(code)` | Answers a pending promotion choice (engine: `Transform`). |
| `session.reset()` | Restarts from the spec's starting position. |

The **view snapshot** is the contract between the engine and any front end:

- board `width`, `height`, and the list of disabled squares;
- every piece: its position, code, and owner;
- the player to move;
- the selected square, if any, and its legal destinations;
- a pending promotion choice, with its options, if any;
- a `status`: playing, or over with a winner or a draw;
- `zones`: for each `POSITION` condition in the spec, its name and the squares it lists per
  player. A zone is plain data in the spec (a player mapped to a fixed list of squares), so
  the wrapper copies it out as it is. No rules are evaluated, and no engine change is needed.
  The front end uses it only to tint squares named in the skin;
- whether the game uses hands/drops (so the front end can say it is unsupported).

The snapshot is the wrapper's own shape, not the engine's `GameState` serialization. The
engine's phase type is still changing (a `Moving { locked }` rework is in progress), and the
snapshot insulates the front end from that.

Turn handling stays entirely in the engine: after a legal move the engine advances the turn,
or keeps the same piece active during a jump continuation, or ends the game. The wrapper
only reports it.

### 2. The board: a web component

A framework-free custom element, `<modern-chess-board>`, so that mdBook can use it with a
script tag and the Solid app can wrap the same element later.

```html
<modern-chess-board spec="examples/march.json" skin="examples/march.skin.json">
</modern-chess-board>
```

- Loads the WebAssembly module once per page, and one `Session` per element.
- Draws the grid, the disabled squares as holes, the pieces, the highlighted legal
  destinations, the selected piece, and a one-line status ("RED to move", "BLACK wins").
- Click a piece of the player to move: select it. Click a highlighted square: move. Click
  elsewhere: deselect. When the engine reports a promotion choice, show a small picker with
  the options.
- Has a reset button.
- Orientation: `y = 0` at the bottom, matching how the specs describe "forward" for the
  first player. (My assumption, to confirm against `chess.json` and `xiangqi.json`.)
- Reads the spec text from a URL given in `spec`, or from an inline
  `<script type="application/json">` child, so a chapter can show a spec it just printed.
- Errors (bad spec, WebAssembly unavailable) render as a short message in place of the board.

### 3. Skins: presentation, kept out of the spec

A skin is a separate JSON file. Proposed shape (builder may refine):

```json
{
  "pieces": {
    "MARCHER": { "RED": "♟", "BLACK": "♙" },
    "RATHA":   { "RED": { "svg": "<svg ...>" }, "BLACK": { "svg": "<svg ...>" } }
  },
  "squares": { "light": "#f0d9b5", "dark": "#b58863" },
  "zones":   { "PROMOTION_ZONE": "#ffe9a8" }
}
```

- A piece entry is a glyph string, or an inline SVG, per player. A piece with a single entry
  for both players gets tinted by player color.
- Zone tints let a chapter show the zone it is explaining (chapters 3 and 6). Zone names come
  from the spec's `conditions`.
- With no skin, or a missing piece entry, the board draws the piece code's first letter in a
  circle colored by owner. A new game is playable immediately.
- The wrapper and engine never read skins.

### 4. Docs integration

- mdBook loads the component script, the stylesheet, and the WebAssembly bundle through
  `additional-js` / `additional-css` in `book.toml` and a small `theme` folder.
- Example specs and skins live under `apps/docs/src/examples/` so that mdBook copies them
  into the built book and the component can fetch them.
- First embeds: chapter 1 (the first game), chapter 2 (pieces), chapter 3 (promotion and
  zones), chapter 9 (jumping). Others follow once the component is stable.
- Build: one documented command builds the WebAssembly bundle and then the book. Build
  output is ignored by git, like `apps/docs/book/` already is.

## Constraints

- The Rust toolchain on the author's machine lacks the `wasm32-unknown-unknown` target and
  `wasm-pack`/`wasm-bindgen`; installing them is part of the setup, and the steps go in the
  docs README.
- No backend. The book remains static files that can be hosted anywhere.
- The docs must stay readable as plain markdown (the chapters work with no board).
- Keep the repo's workflow: tests first, conventional commits, work on a branch.

## Decisions already made

- Looks are separate from the spec, in a skin file (the author decided this).
- The board is clickable with automatic turn switching (the author decided this).
- Skins are fully optional (my call, to keep new games one step away from playable).
- Zone tints come from the snapshot's `zones`, copied from the spec's `POSITION` conditions
  (my call, after review: zones are data, so this is not rules logic).
- Baseline: the wrapper is built against the committed engine on `main`. When the
  `Moving { locked }` rework lands, only the wrapper's mapping from engine phase to snapshot
  changes (my call). The engine may gain small **additive** public accessors if the snapshot
  needs them (for example reading pending promotion options or the jump lock), but no change
  to engine behaviour. Anything beyond that is a separate, reviewed change.
- Hand games (`hand_enabled`): the board renders **read-only** with a notice that drops are
  not supported yet (my call). A board that allowed moves while captured pieces vanished into
  a hand that cannot be played would teach wrong rules. `select` and `move` refuse in this
  mode, and the snapshot reports it.

## Left to the builder

- Choice of `wasm-pack` or plain `wasm-bindgen` plus a Makefile target, and the bundle
  layout.
- Exact skin schema details, and how the fallback pieces are styled.
- Whether the component renders with SVG or CSS grid.
- Responsive sizing and accessibility (keyboard selection, labels).
- How to test: engine-level tests for the view snapshot are required; browser-level checks
  are up to the builder.
- Which scratch specs and skins to add for the first embeds.
