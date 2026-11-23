# The docs book

An [mdBook](https://rust-lang.github.io/mdBook/) that teaches the Modern Chess game spec.
Some chapters embed a **playable board** that runs the real engine in the browser.

## Build it

One-time prerequisites:

```sh
rustup target add wasm32-unknown-unknown
brew install wasm-pack mdbook node   # Node 22 or newer
```

Then, from anywhere:

```sh
apps/docs/build.sh
```

That compiles `crates/wasm` to WebAssembly (into `theme/pkg/`, git-ignored), gathers the specs
next to their skins in `src/boards/`, builds the book into `book/`, and puts the board script
beside the book's own files. (The first run may download `wasm-opt` through `wasm-pack`.)

The WebAssembly step alone is `apps/docs/build-engine.sh`. The web app loads the same bundle,
and its `pnpm dev` and `pnpm build` run that script first, so it never plays an old engine.

## Read it

The boards load the engine and specs with `fetch`, so open the book over HTTP, not `file://`:

```sh
python3 -m http.server -d apps/docs/book 4173
# then open http://localhost:4173
```

(`mdbook serve` rebuilds the text but not the board script. Re-run `build.sh` after changing
Rust or anything in `theme/`.)

## Add a board to a chapter

A board needs a spec in `src/boards/`. There are three ways to get one:

- **The real game, unchanged.** `build.sh` copies `examples/march.json`,
  `examples/chaturanga_pieces.json` and the engine's `checkers.json`, `ataxx.json`, `go.json` and `pong_hau_ki.json` there. To use
  another shipped spec, add it to that `cp` line in `build.sh` (and to `.gitignore`).
- **A prepared position of a real game.** Add a recipe to `boards.derived.json`: a base spec
  from `crates/engine/specs/`, plus the board, zones and starting layout to replace. Everything
  else (pieces, moves, rules) comes from the base spec, so the board cannot drift from the game.
  `scripts/derive-boards.mjs` writes the result to `src/boards/<out>.json`. Generated files are
  git-ignored; add the new name to `.gitignore`.
- **Your own small spec.** Commit it in `src/boards/`.

Then add the element to the chapter. The `spec` and `skin` paths are relative to the chapter's
page (chapters sit at the top of the book, so `boards/...` works):

```html
<modern-chess-board spec="boards/march.json" skin="boards/chess.skin.json">
  Playable board (needs JavaScript).
</modern-chess-board>
```

`skin` is optional. Instead of `spec`, you can put the spec inline in a
`<script type="application/json">` child.

What a reader can do: answer what the game asks, one click at a time. Click a piece, and its
squares are marked. Click one, and the move is made. A promotion with a choice asks which piece.
Turns switch automatically, a jump in progress keeps its piece chosen, the status line announces a
win or a draw, and a pass is a button. In a game with a hand (Shogi, Crazyhouse) the pieces the
player to move holds are shown beside the board: click one, then a place, to drop it.

## Skins

A **skin** says how a game *looks*; the spec says how it *plays*. They are separate on purpose.
A skin is a JSON file, and everything in it is optional:

```json
{
  "pieces": {
    "RATHA":  { "WHITE": "♖", "BLACK": "♜" },
    "PADATI": "♟",
    "ASHVA":  { "WHITE": { "svg": "<svg viewBox='0 0 10 10'>…</svg>" }, "BLACK": "♞" }
  },
  "squares": { "light": "#f0d9b5", "dark": "#b58863" },
  "zones":   { "REACH_END": "#f3d98b" }
}
```

- A piece entry is a glyph, or `{ "svg": "…" }`. Give one entry for both players (it is tinted
  by owner) or one per player name.
- `zones` colours the squares of a zone from the spec (an `IN_ZONE` test names it), by its name.
- A piece the skin does not mention is drawn as its first letter in a circle coloured by owner,
  so a new game is playable straight away. A missing or broken skin file is ignored.

## How it fits together

| Piece | Where | Job |
|-------|-------|-----|
| Engine | `crates/engine` | All rules. |
| Session | `crates/session/src/session.rs` | Wraps the engine; names every position (`"4,2"`) and returns a stable JSON view listing the board's nodes, with the `choice` being asked. Tested with `cargo test -p modern-chess-session`. |
| Bindings | `crates/wasm/src/lib.rs` | `wasm-bindgen` shim exposing `Board`: `view()`, `choose(answer)` (an answer is `{"place": ...}`, `{"code": ...}`, `{"held": ...}` or `{"action": ...}`), `cancel()` and `reset()`. Tested with `cargo test -p modern-chess-wasm`. |
| Element | `theme/modern-chess-board.js` | The `<modern-chess-board>` element. With a `remote` attribute it loads no engine: the web app gives it a `view` and hears `choose` and `cancel` events. |
| Helpers | `theme/board-model.js` | Skin handling. Tested with `node --test apps/docs/theme/test/board-model.test.mjs`. |
| Recipes | `boards.derived.json`, `scripts/derive-boards.mjs` | Prepared positions made from real specs. Tested with `node --test apps/docs/scripts/derive-boards.test.mjs`. |
| Board tests | `crates/session/tests/boards.rs` | Plays each embedded board (castling, en passant, the hill, stalemate, the blast). Needs the boards generated first: run `apps/docs/build.sh`. Run with `cargo test -p modern-chess-session --test boards`. |

`node apps/docs/theme/test/smoke.mjs` plays moves through the compiled bundle: a piece chosen and
moved, a cancel, and a capture dropped back from the hand. Run it after `apps/docs/build.sh`.
