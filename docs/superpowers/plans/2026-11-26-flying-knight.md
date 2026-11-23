# The Flying Knight Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `ATTACKED` can look two moves ahead, end checks can be limited to players, and the Flying Knight is in the catalogue, written only with those and the format's existing blocks.

**Architecture:** `ATTACKED` gains `moves` (1 or 2); a two-move reading is a new `Game::reaches_in_two` beside `reaches` (first move to a threatened square, capture included, then a one-move reach on that view). `EndCheckSpec` gains `players`. The game is `crates/engine/specs/flying_knight.json`, a copy of `chess.json` where Black's turn is one `DOUBLE_MOVE` action and White's check rules use `moves: 2`.

**Tech Stack:** Rust (serde, serde_json), `cargo test`; docs are Markdown (`specs/SPEC_FORMAT.md`).

**Spec:** `docs/superpowers/specs/2026-11-26-flying-knight-design.md`

## Global Constraints

- No change to what any existing spec means: `moves` defaults to 1, `players` to everyone.
- `moves` is 1 or 2 only, and `moves: 2` is refused together with `keeping`. `players` on an end check names players of the game; an empty list is refused (it applies to none), as `every_action`'s `for` is.
- Errors follow the existing style of `GameSpecError::InvalidTest` and `InvalidEndCheck`-like variants; look at how `keeping` and action `players` are refused and match them.
- Every fix and feature is proved by a test seen to fail first.
- Commits are conventional and end with `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. Branch is `feature/flying-knight` (holds the design). Do not push or open a PR without being asked.
- Suites: `cargo test -p modern-chess-engine -p modern-chess-session -p modern-chess-wasm` after every task; `cargo test --workspace` before the end (`TEST_DATABASE_URL`, dev Postgres on `localhost:5455`). Clippy warnings (`cargo clippy -p modern-chess-engine -p modern-chess-session --all-targets`) must not rise from the count on `main` (measure it first). `cargo fmt --check` clean.
- macOS: `sed -i ''`; there is no `timeout`.

## Review Focus

1. A king that Black reaches only by a first move that takes a White piece (the capture counts as the first move) is attacked in two (Task 1 test).
2. A square Black needs three moves to reach is not attacked in two, and a blocked slider's first move is limited to the threatened squares: no overreach (Task 1 tests).
3. `moves: 2` and `players` are refused where they make no sense (Tasks 1 and 2 tables), and an existing spec's reading is unchanged: the catalogue counts still pass.
4. Black may end its first move in check and must end its second out of it; a first move that leaves no safe second move is not offered as a way ending in check (Task 3).
5. A Black stalemate is a draw, a Black mate is a loss, and White's mate and stalemate use the two-move reading (Task 3).

---

### Task 1: `ATTACKED` with `moves`

**Files:**
- Modify: `crates/engine/src/specs/game/test_block.rs` (`TestSpec::Attacked`), `specs/game/game.rs` (its validation, near the `keeping` check), `logic/rules/test_block/mod.rs`, `build.rs`, `judge.rs` (`Test::Attacked`, `attacked`), `logic/pieces/attack.rs` (`Game::reaches_in_two`)
- Create: `crates/engine/src/tests/look_ahead/mod.rs`; add `mod look_ahead;` to `tests/mod.rs` (alphabetical)
- Modify: `specs/SPEC_FORMAT.md` (the `ATTACKED` row and its error-table rows)

**Interfaces:**
- Produces: `TestSpec::Attacked { r#in, by, keeping, moves: Option<u8> }` (`#[serde(default, skip_serializing_if = "Option::is_none")]`); `Test::Attacked { .., moves: u8 }`; `Game::reaches_in_two(&self, attacker: &str, pieces: PiecesView<'_>, places: &[&Position]) -> bool`.
- Consumes: `Game::reaches`, `Blueprint::visit_threats`, `BoardChange::set_piece(pos, piece)` and `BoardChange::clear(&pos)`, `PiecesView::with(base, &changes)`.

- [ ] **Step 1: Write the failing tests** in `look_ahead/mod.rs` on a small inline spec (8×8, a White `WALKER` that steps one place east with an `every_action` entry `NOT ATTACKED in BOARD where ALLY WALKER by ENEMY` with `moves` as a parameter, and Black pieces `KNIGHT` and `ROOK` with their usual moves). Names and assertions:
  - `a_knights_two_jumps_reach_what_one_does_not`: with `moves: 2` a walker step onto a square a Black knight reaches in two jumps is not offered; with `moves` absent or 1 it is.
  - `a_square_three_moves_away_is_not_attacked_in_two`: a square the knight needs three jumps for is offered with `moves: 2`.
  - `the_first_move_may_take_a_piece_and_still_attack`: a Black rook that can attack the walker's square only after its first move captures a White piece (the capture is the first move) makes `moves: 2` refuse it; with the White piece absent and the rook's other first moves unable to reach, it is offered. Pick the squares by hand and say why in a comment.
  - `an_attacker_without_a_first_move_attacks_in_one_only`: a Black piece boxed in by its own pieces on every threatened square attacks nothing in two.
  - `moves_must_be_one_or_two`, `moves_two_is_refused_with_keeping`: the spec is refused (`parse_game_spec_str`), naming `moves`.
  - `an_existing_spec_reads_the_same_with_moves_one`: chess with `moves: 1` written on its first `ATTACKED` plays the same first-ply move count as without.
- [ ] **Step 2: Run** `cargo test -p modern-chess-engine look_ahead` → all FAIL (the key is unknown, `unknown field moves`).
- [ ] **Step 3: Implement.** Spec field and validation (1 or 2; refused with `keeping` when 2). Compile it to `Test::Attacked.moves` (default 1). In `judge.rs`'s `attacked`, for `moves == 2` call `Game::reaches_in_two`, otherwise `reaches` as before. `reaches_in_two`: true when `reaches(attacker, pieces, places, None)` holds; otherwise for each piece of `attacker` at `from`, visit its threats with `visit_threats`; for each threatened square `to`, build the changes `[clear(from), set_piece(to, piece.clone())]` (this takes whatever stood on `to`), the view `PiecesView::with(pieces.base, &changes)` — add a helper on `PiecesView` that lays further changes over a view if `with` cannot nest — and test `reaches` on it; stop at the first true. Attack sets as `reaches` reads them: conditions on moves are ignored (documented).
- [ ] **Step 4: Run** `cargo test -p modern-chess-engine look_ahead` → PASS, then the engine, session and wasm suites. Update `SPEC_FORMAT.md`: the `ATTACKED` row gains `"moves": 1`, with a paragraph (one move of the attacker, or two in a row: the attacker takes a first move to a square it threatens, capturing what stands there, then any piece threatens; no reply between), and the refusals in the error table.
- [ ] **Step 5: Commit** `feat(engine): ATTACKED can look two moves ahead`.

---

### Task 2: End checks limited to players

**Files:**
- Modify: `crates/engine/src/specs/game/end_check.rs` (`EndCheckSpec.players`), `specs/game/game.rs` (validate the names; refuse an empty list), `logic/rules/ends.rs` (`EndCheck.players`, `check_end_checks`)
- Modify tests: `crates/engine/src/tests/end_checks/mod.rs` (or a new `tests/end_check_players/mod.rs`)
- Modify: `specs/SPEC_FORMAT.md` (the end check section and error table)

**Interfaces:**
- Produces: `EndCheckSpec.players: Option<Vec<String>>` (`#[serde(default, skip_serializing_if = "Option::is_none")]`; absent means everyone); `EndCheck.players: Option<Vec<String>>`, compared against the viewpoint player (`check.next` picks `current_player` or `previous_player`).

- [ ] **Step 1: Write the failing tests.** `an_end_check_for_one_player_is_judged_only_for_them`: a two-player game whose only end check is `{ "as": "NEXT", "players": ["BLACK"], "when": { "test": "STUCK" }, "then": { "outcome": "LOSE" } }`: White stuck is not lost (the game goes on to the default draw), Black stuck loses. `an_end_check_names_players_of_the_game` (a name that is no player is refused naming it) and `an_end_check_with_no_players_is_refused` (`"players": []`).
- [ ] **Step 2: Run** → FAIL (unknown field `players`).
- [ ] **Step 3: Implement** the field, the validation (reuse the check of action players at `game.rs` near line 939 for the message style), the compiled field, and in `check_end_checks` skip a check whose `players` lacks the viewpoint player before judging (and before computing the ways).
- [ ] **Step 4: Run** the suites → PASS. Document in `SPEC_FORMAT.md`.
- [ ] **Step 5: Commit** `feat(engine): an end check can be limited to players`.

---

### Task 3: The Flying Knight

**Files:**
- Create: `crates/engine/specs/flying_knight.json`, `crates/engine/src/tests/flying_knight/mod.rs` (`mod flying_knight;` in `tests/mod.rs`)
- Modify: `crates/engine/src/tests/catalogue_counts/mod.rs` (a row `flying_knight: 3 => N`, N recorded from the first run), `specs/SPEC_FORMAT.md` or `apps/docs/src/05-winning-and-drawing.md` where the variants are listed, if they list chess variants
- Not done here (say so in the final report): a page for the web catalogue (`crates/game/pages`) and the seed list: product decisions.

**Interfaces:**
- Consumes: `ATTACKED moves`, end check `players`, action `players`, `every_action.for`.

- [ ] **Step 1: Write the failing tests** in `flying_knight/mod.rs`, loading `./specs/flying_knight.json` (set the position by inserting pieces into `game.state.pieces`, then `game.work_out_ways()` and play answers with `game.choose(Answer::Place(..))`, as `not_attacked` and `castling_rules` do):
  - `the_start_is_chess_for_white_and_black_has_a_king_and_a_knight`: White has 16 pieces, Black two (the king on e8 and the knight on g8), White to move with the usual 20 moves.
  - `black_takes_two_moves_in_a_turn`: after one White move, Black's turn takes four answers (piece, target, piece, target); after the fourth the turn passes to White; the history records one Black turn.
  - `black_may_step_into_check_with_the_first_move_and_out_with_the_second`: a position where Black's first move steps onto an attacked square and the second leaves it: the way exists. A position where every second move leaves the king attacked: no way starts with that first move.
  - `white_may_not_leave_its_king_open_to_two_black_moves`: a White move that lets the knight, after one jump, give check (the king taken on its second move) is not offered; the same position with the walker elsewhere offers it.
  - `black_stuck_in_check_loses`, `black_stuck_not_in_check_draws`, `white_stuck_in_two_move_check_loses`, `white_stuck_not_in_check_draws`: four constructed positions, each reaches `GameOver` with the winner as written (a draw is `winner: None`).
  - `white_may_castle_when_black_cannot_reach_the_squares_in_two`, `white_may_not_castle_through_a_square_black_reaches_in_two`.
- [ ] **Step 2: Run** `cargo test -p modern-chess-engine flying_knight` → FAIL (no such spec).
- [ ] **Step 3: Write the spec.** Copy `chess.json` and edit with a throwaway script in `$CLAUDE_JOB_DIR/tmp` (do not commit it): name `FLYING_KNIGHT`; Black's starting positions keep only the king (e8) and the knight (g8); `actions`: `MOVE` with `"players": ["WHITE"]` and a new `DOUBLE_MOVE` with `"players": ["BLACK"]` whose `do` is `choose PIECE (ALLY), choose MOVE, choose PIECE (ALLY), choose MOVE`; the `every_action` entries: the king-safety entry gets `"for": ["MOVE"]` with its `ATTACKED` as `"moves": 2`, and a copy with `"for": ["DOUBLE_MOVE"]` with the plain `ATTACKED`; the QUIET counter entry applies to both actions; castling's `ATTACKED` tests on the king's squares (out of, through) get `"moves": 2`; the mate end check becomes two entries, `{ "as": "NEXT", "players": ["WHITE"], ... "moves": 2 ... LOSE }` and `{ "as": "NEXT", "players": ["BLACK"], ... plain ... LOSE }`, then the stalemate `DRAW`; pieces Black does not have stay in the piece list (White has them). Check that the second `PIECE` choice of a double move offers the pieces as the first move left them (the ways are worked out on the progressive board). Fix the spec until the Step 1 tests pass; keep promotion and en passant as in chess (they do not arise for Black).
- [ ] **Step 4: Run** the new tests, then the whole engine suite, the `catalogue_counts` row (record N from its failure), and the session and wasm suites. Add the game to the docs where chess variants are listed, if there is such a list.
- [ ] **Step 5: Commit** `feat(engine): the Flying Knight`.

---

### Final

- [ ] Run `cargo test --workspace` with `TEST_DATABASE_URL`, `cargo clippy`, `cargo fmt --check`; update `docs/superpowers/specs/2026-11-23-remaining-work.md` (the Flying Knight done, `ATTACKED` `moves` and end-check `players` noted, the look-ahead bullet resolved); final whole-branch review by an Opus subagent per the executing-plans skill; report rulings and deferred minors.
