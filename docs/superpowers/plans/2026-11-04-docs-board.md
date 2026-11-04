# Playable Docs Boards Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Put a live, clickable board next to the docs' examples, driven by the real engine compiled to WebAssembly, with looks supplied by separate skin files.

**Architecture:** A new crate `crates/wasm` holds a pure-Rust `Session` (testable natively) that wraps `Game` and returns a stable JSON `View`; a thin `wasm-bindgen` shim exposes it. A framework-free `<modern-chess-board>` custom element loads the bundle, a spec and an optional skin, and draws the view. mdBook loads it through `additional-js`/`additional-css`.

**Tech Stack:** Rust (workspace crate `crates/wasm`, `wasm-bindgen`, `serde_json`), `wasm-pack` (`--target web`), vanilla JS (ES modules, `node --test` for pure helpers), mdBook.

**Spec:** `docs/superpowers/specs/2026-11-04-docs-board-design.md`

## Global Constraints

- The board never decides legality; every rule answer comes from the engine.
- No presentation fields in the spec format; skins are separate JSON files.
- Skins are optional; a missing skin or missing piece entry falls back to the piece code's first letter in a circle coloured by owner.
- Hand games (`hand_enabled`) are read-only with a "drops not supported yet" notice; `select`/`move` refuse.
- Engine changes limited to `parse_spec_str` and small additive read accessors. No behaviour changes.
- Built against committed `main`; the view is the wrapper's own shape, never the engine's `GameState` serialization.
- `y = 0` is drawn at the bottom. Positions are `[x, y]`; `dimensions` is `[width, height]`.
- Tests first; conventional commits ending with `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`; local commits only, no push.
- One documented command builds the bundle and the book; build output is git-ignored; a failed WebAssembly load shows a readable message and leaves the chapter intact.

## Review Focus

- **Spec fails to parse or validate** (bad JSON, unknown piece code): `Session::new` returns `Err` with the engine's message; the element shows it instead of a board. Pinned in Task 2 and 6.
- **Click on an empty square, an opponent piece, or a non-destination**: returns `Err`, view unchanged, no panic. Pinned in Task 3.
- **Moves after the game is over**: refused, and the status stays `won`/`draw`. Pinned in Task 3.
- **Board that is not 2-D** (`dimensions.len() != 2`): `Session::new` returns `Err("only 2-D boards are supported")`. Pinned in Task 2.
- **Skin missing, 404, malformed, or lacking a piece**: board still renders with fallback pieces. Pinned in Task 5.

---

### Task 1: Parse a spec from text

**Files:**
- Modify: `crates/engine/src/specs/game/parser.rs`, `crates/engine/src/specs/mod.rs`
- Create: `crates/engine/src/tests/spec_str/mod.rs`; register `mod spec_str;` in `crates/engine/src/tests/mod.rs`

**Interfaces:**
- Produces: `pub fn parse_spec_str(contents: &str) -> Result<GameSpec, GameSpecError>` re-exported as `crate::specs::parse_game_spec_str`. `parse_spec` calls it.

- [ ] **Step 1: Write failing tests** in `tests/spec_str/mod.rs`: `parses_a_valid_spec_from_text` (`include_str!("../../../specs/chaturanga.json")` parses and `.name` is non-empty), `rejects_malformed_json` (`"{"` is `Err`), `rejects_unknown_piece_in_starting_position` (chaturanga text with one starting piece code replaced by `"NOPE"` is `Err`).
- [ ] **Step 2:** `cd crates/engine && cargo test spec_str` — expect compile error (function missing).
- [ ] **Step 3:** Implement `parse_spec_str`: `serde_json::from_str`, then `validate_specs()`; make `parse_spec` read the file and delegate. Export from `specs/mod.rs`.
- [ ] **Step 4:** `cargo test` in `crates/engine` — expect all pass (previous count plus 3).
- [ ] **Step 5:** Commit `feat: parses game specs from a string`.

### Task 2: Session scaffold and the view snapshot

**Files:**
- Create: `crates/wasm/Cargo.toml` (package `modern-chess-wasm`, `crate-type = ["cdylib", "rlib"]`, deps: engine as `modern-chess-engine`, `serde` derive, `serde_json`, `wasm-bindgen`), `crates/wasm/src/lib.rs` (`pub mod session;`), `crates/wasm/src/session.rs`, `crates/wasm/src/view.rs`
- Modify: root `Cargo.toml` (add `crates/wasm` to `members`)
- Test: `crates/wasm/tests/view.rs`

**Interfaces:**
- Produces in `view.rs` (all `Serialize`, camelCase not needed; use these exact snake_case names): `View { width: u8, height: u8, disabled: Vec<[u8;2]>, players: Vec<String>, pieces: Vec<PieceView>, turn: String, selected: Option<[u8;2]>, destinations: Vec<[u8;2]>, promotion: Option<Vec<String>>, status: Status, zones: BTreeMap<String, BTreeMap<String, Vec<[u8;2]>>>, hands: bool }`; `PieceView { x: u8, y: u8, code: String, player: String }`; `Status` serialised with `#[serde(tag = "state", rename_all = "lowercase")]`: `Playing`, `Won { winner: String }`, `Draw`.
- Produces in `session.rs`: `pub struct Session { game: Game, spec: GameSpec }` (keep the parsed spec so `reset` rebuilds the game); `Session::new(spec_json: &str) -> Result<Session, String>`; `view(&self) -> View`; `reset(&mut self)`.
- Pieces sorted by `(y, x)`; `zones` built from `game.conditions` entries whose `type == "POSITION"`, parsing each `"x,y"` string (invert with `shared::into_position` or `split(',')`), sorted.
- Selection/destinations/promotion/status come from `state.phase` (`Moving`/`Jumping` → selected + destinations from `available_moves`; `Transforming` → `promotion`; `GameOver{winner}` → `Won`/`Draw`).

- [ ] **Step 1: Write failing tests** in `tests/view.rs` using `include_str!` on the engine specs: `view_of_chess_start` (`width == 8`, 32 pieces, `turn == "WHITE"`, `status == Playing`, `selected == None`, `hands == false`); `view_reports_disabled_squares` (`4player_chess.json` has non-empty `disabled`); `view_reports_zones` (chess's `REACH_END` zone present with `WHITE` squares); `view_flags_hand_games` (`shogi.json` → `hands == true`); `new_rejects_bad_json` (`"{"` → `Err`); `new_rejects_non_2d_board` (inline spec with `"dimensions": [3,3,3]` → `Err` containing `2-D`); `reset_restores_start` (after construction `reset()` then `view()` equals initial view as JSON).
- [ ] **Step 2:** `cargo test -p modern-chess-wasm` — expect compile failure (crate empty).
- [ ] **Step 3:** Implement the types and `Session::new/view/reset`. `new` uses `parse_game_spec_str`, rejects non-2-D dimensions, builds `Game::from_spec(spec.clone())`; errors are `format!("{e:?}")` of the engine error.
- [ ] **Step 4:** `cargo test -p modern-chess-wasm` — expect pass.
- [ ] **Step 5:** Commit `feat: adds the wasm crate with a stable view snapshot`.

### Task 3: Playing through the session

**Files:**
- Modify: `crates/wasm/src/session.rs`
- Create: `crates/wasm/tests/play.rs`, fixtures `crates/wasm/tests/fixtures/promote.json` (2-D board with one pawn one step from promotion and both leaders placed, copy the shape from `apps/docs/src/03-promotion-and-zones.md`'s final spec) and `bare.json` (two-piece game whose first capture ends the game via `OPPONENT_BARE`, shape from chapter 5)

**Interfaces:**
- Produces: `Session::select(&mut self, x: u8, y: u8) -> Result<(), String>`; `Session::make_move(&mut self, x: u8, y: u8) -> Result<(), String>`; `Session::promote(&mut self, code: &str) -> Result<(), String>`.
- Rules: `select` and `make_move` return `Err("drops are not supported yet")` when `game.hand_enabled`; every call returns `Err` once phase is `GameOver`; `select` returns `Err` while `Transforming`; `select` uses `GameTransition::CalculateMoves` and clears selection state if the engine errors; `make_move` uses `ExecuteMove`; after a capture that leaves phase `Jumping`, the view already holds the continuation destinations (the engine sets them), so nothing extra is called. Errors never change state.

- [ ] **Step 1: Write failing tests** in `tests/play.rs`: `selecting_a_pawn_lists_its_moves` (chess: select `(4,1)` → destinations `[[4,2],[4,3]]`, `selected == Some([4,1])`); `moving_switches_the_turn` (then `make_move(4,3)` → `turn == "BLACK"`, piece at `(4,3)`, `selected == None`); `cannot_select_the_opponents_piece` (select `(4,6)` on White's turn → `Err`, view unchanged); `cannot_move_to_a_non_destination` (select `(4,1)`, `make_move(0,5)` → `Err`, selection still `(4,1)`); `selecting_an_empty_square_errors` (`(4,4)`); `promotion_flow` (fixture: move the pawn → `promotion == Some(options)` with the spec's options, `select` errors, `promote(code)` → piece code changed, turn switched, `promotion == None`); `promote_rejects_unknown_option`; `jump_keeps_the_piece_selected` (`checkers.json`: arrange via the real opening so one capture with a follow-up jump is available, or use a fixture; assert `selected` is the landing square and `destinations` non-empty, `turn` unchanged); `game_over_is_reported_and_final` (`bare.json`: capture → `status == Won`, then `select`/`make_move` return `Err`); `hand_games_are_read_only` (`shogi.json`: `select` of any square returns `Err` containing `drops`).
- [ ] **Step 2:** `cargo test -p modern-chess-wasm --test play` — expect failures (methods missing).
- [ ] **Step 3:** Implement the three methods per the rules above. Add `pub fn` read accessors on `Game` only if the view needs something private.
- [ ] **Step 4:** `cargo test` at the workspace root — expect all green, including the engine's.
- [ ] **Step 5:** Commit `feat: lets a session select, move and promote`.

### Task 4: WebAssembly bindings and the build

**Files:**
- Modify: `crates/wasm/src/lib.rs`, `.gitignore`
- Create: `apps/docs/build.sh` (executable), `apps/docs/theme/test/smoke.mjs`

**Interfaces:**
- Produces: `#[wasm_bindgen] pub struct Board` wrapping `Session` with `#[wasm_bindgen(constructor)] new(spec_json: &str) -> Result<Board, JsError>`, `view() -> String` (JSON), `select(x: u8, y: u8) -> Result<(), JsError>`, `make_move(x: u8, y: u8) -> Result<(), JsError>`, `promote(code: &str) -> Result<(), JsError>`, `reset()`. JS sees class `Board`.
- `build.sh`: runs `wasm-pack build ../../crates/wasm --target web --out-dir ../../apps/docs/theme/pkg --release`, then `mdbook build`. Prints the prerequisite install commands if `wasm-pack` or the target is missing. `.gitignore` gains `/apps/docs/theme/pkg/`.

- [ ] **Step 1: Install prerequisites:** `rustup target add wasm32-unknown-unknown` and `brew install wasm-pack` (ask the user if either is denied).
- [ ] **Step 2: Write the failing smoke test** `smoke.mjs`: `initSync` with the bytes of `theme/pkg/*_bg.wasm`, build a `Board` from the chess spec text, `select(4,1)`, `make_move(4,3)`, parse `view()`, assert `turn === "BLACK"` and a piece at `(4,3)`; also assert `new Board("{")` throws.
- [ ] **Step 3:** `node apps/docs/theme/test/smoke.mjs` — expect failure (bundle missing).
- [ ] **Step 4:** Add the bindings and `build.sh`; run `apps/docs/build.sh` (or just the `wasm-pack` line first).
- [ ] **Step 5:** `node apps/docs/theme/test/smoke.mjs` — expect pass.
- [ ] **Step 6:** Commit `feat: compiles the session to WebAssembly`.

### Task 5: Pure board helpers (skins, fallback, geometry)

**Files:**
- Create: `apps/docs/theme/board-model.js`, `apps/docs/theme/test/board-model.test.mjs`

**Interfaces:**
- Produces (ES module exports): `normalizeSkin(raw: unknown): Skin` (never throws; returns `{ pieces: {}, squares: {light, dark}, zones: {} }` with defaults for anything absent or malformed); `pieceFace(skin, code, player, players): { kind: "glyph" | "svg" | "fallback", value: string, colour?: string }` (a skin entry per player wins; a single non-per-player entry applies to everyone and is tinted by player; otherwise `fallback` with `value = code[0]` and a colour taken from `players.indexOf(player)` in a fixed palette); `cellRect(view, x, y): { col: number, row: number }` with `row = view.height - 1 - y`; `zoneTint(skin, view, x, y): string | null` (first zone named in the skin containing `[x,y]` for any player).
- [ ] **Step 1: Write failing tests** with `node:test`: glyph per player; SVG entry; shared entry tinted; missing piece → fallback letter and distinct colours for two players; `normalizeSkin(null)`, `normalizeSkin("x")`, `normalizeSkin({pieces: 5})` all return defaults; `cellRect` puts `y=0` on the bottom row; `zoneTint` returns the colour for a zone square and `null` outside.
- [ ] **Step 2:** `node --test apps/docs/theme/test/` — expect failure.
- [ ] **Step 3:** Implement the four functions.
- [ ] **Step 4:** `node --test apps/docs/theme/test/` — expect pass.
- [ ] **Step 5:** Commit `feat: adds skin and geometry helpers for the board`.

### Task 6: The `<modern-chess-board>` element

**Files:**
- Create: `apps/docs/theme/modern-chess-board.js`, `apps/docs/theme/modern-chess-board.css`

**Interfaces:**
- Consumes: `Board` from `./pkg/<name>.js` (default export `init`), helpers from `./board-model.js`.
- Produces: custom element `modern-chess-board` with attributes `spec` (URL), `skin` (URL, optional); or an inline `<script type="application/json">` child holding the spec when `spec` is absent. Renders a status line, a CSS-grid board (disabled squares as empty holes, zone tints, pieces, destination dots, selected outline), a promotion picker when `promotion` is set, and a Reset button. Click logic: own piece → `select`; highlighted destination → `make_move`; elsewhere → ignore (engine errors are shown briefly in the status line, never thrown). Hand games show the notice. Any load error (spec fetch, WebAssembly, `new Board`) replaces the board with `<p class="mcb-error">…</p>`. Module is loaded once; `init` is awaited once and shared.
- [ ] **Step 1:** Implement the element. Load the module through a shared promise so several boards on one page do not re-initialise.
- [ ] **Step 2: Verify in a browser** (Chrome tools): build the book (Task 7 wiring may be needed first, so do this step after Task 7 Step 3 if the page is not served yet) and check on a page: clicking White's e-pawn lights two squares; clicking one moves it and the status reads `BLACK to move`; clicking a Black piece first does nothing harmful; Reset restores; a broken `spec` URL shows the error message and the rest of the page is intact.
- [ ] **Step 3:** Commit `feat: adds the modern-chess-board element`.

### Task 7: Docs integration and first embeds

**Files:**
- Modify: `apps/docs/book.toml` (`additional-js = ["theme/board-loader.js"]`, `additional-css = ["theme/modern-chess-board.css"]`), chapters `01-first-game.md`, `02-pieces-that-move.md`, `03-promotion-and-zones.md`, `09-forced-captures.md`
- Create: `apps/docs/theme/board-loader.js` (classic script that `import()`s `modern-chess-board.js` relative to itself), `apps/docs/src/boards/` containing one spec per embed taken from the chapter's final example (`march.json` copied from `apps/docs/examples/march.json`, `chaturanga_pieces.json` likewise, plus `promotion.json` and `jumping.json` extracted from chapters 3 and 9) and a skin per board (`*.skin.json`; at least one board with a skin, at least one without), `apps/docs/README.md` (prerequisites, the one build command, how to add a board and a skin, the skin schema)
- mdBook copies non-markdown files under `src/`, which is why specs and skins live in `src/boards/`; theme files outside `src/` are copied through `additional-js/css` and the loader's relative imports must be verified in the built output (`pkg/` is not auto-copied: have `build.sh` copy `theme/pkg` into `book/theme/pkg`, or list its files in `additional-js`).

- [ ] **Step 1:** Add the loader and `book.toml` entries; run `apps/docs/build.sh`; confirm `book/` contains the element script, the CSS, the bundle and `boards/*.json`.
- [ ] **Step 2:** Embed `<modern-chess-board spec="boards/march.json">` (path relative to each chapter page) in chapter 1 after the final spec, and the matching boards in chapters 2, 3 and 9, each with one sentence telling the reader what to try.
- [ ] **Step 3: Verify in a browser:** open each chapter from `book/`, serve over HTTP (`python3 -m http.server` in `book/`); play one move on each board; chapter 3's promotion picker appears; chapter 9's second jump keeps the piece selected; the page still reads fine with scripts disabled.
- [ ] **Step 4:** Run everything: `cargo test`, `node --test apps/docs/theme/test/`, `node apps/docs/theme/test/smoke.mjs`; expect all green.
- [ ] **Step 5:** Commit `docs: adds playable boards to chapters 1, 2, 3 and 9`.
