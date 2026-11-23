# Rule blocks, slice 3: rules that apply to every move — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Check, no check on the opponent, forced captures, liberty capture and repeated
positions are written in specs as blocks (`every_action`, `must` and a few small tests, regions
and filters), and the six special forms leave the engine and the format.

**Architecture:**
- **New blocks first, beside the old forms.** Tasks 1–4 add the tests, regions, filters,
  `every_action` and `must`. Nothing is rewritten yet, so every task ends green.
- **One rule at a time moves over.** Check (Task 5), forced captures (Task 6) and Go (Task 7):
  rewrite the specs that use the form, delete the engine code, refuse the old key with
  `OldForm`.
- **Then docs and the proof** (Tasks 8–9).
- **The mechanism is slice 2's.** An action's steps run on the changing position, and a failed
  test removes that way. `every_action` steps are appended to each action's steps when the spec
  is compiled, so they run last.

**Tech Stack:** Rust (the engine crate `modern-chess-engine`; the session, wasm and game crates
only read the changed `Rules` fields through the engine), the mdBook book in `apps/docs`.

**Spec:** `docs/superpowers/specs/2026-11-22-every-move-rules-design.md`. Read it whole. It builds
on `docs/superpowers/specs/2026-11-22-rule-blocks-design.md` (slice 1's tests, regions, places,
filters) and `docs/superpowers/specs/2026-11-22-actions-and-effects-design.md` (steps, ways).

## Global Constraints

- **JSON shapes exactly as the design's tables; nothing else is accepted.** Every block refuses
  unknown fields (`deny_unknown_fields`), as slice 1's and 2's do.
  - `every_action`: `[ { "for": [action names]?, "do": [steps] } ]`. `do` holds tests, effects
    and `if`; a `choose` is refused when the spec is read.
  - `must`: `[ TEST ]`. Each entry narrows the ways in order; if no way passes an entry, it
    changes nothing.
  - Tests: `{ "test": "REMOVED", "where": F }`, `{ "test": "REPEATS" }`, and `ATTACKED` with
    `"by": "ENEMY"` (default) or `"MOVER"`.
  - Regions: `{ "region": "BOARD", "where": F }`, `{ "region": "GROUP", "of": P, "through":
    "orthogonal", "where": F }`, and `AROUND` whose `of` is a place or a region.
  - Filter: `{ "empty": true }`, alone (no other filter field); `COUNT` with it counts empty
    places.
- **Behaviour is unchanged except what the design names:** a Go capture now carries the capture
  mark in the history; the order of the stones a Go move removes may differ.
- **`leader` stays in specs** for one more slice. After this slice it filters no move; only the
  no-legal-move outcome and Three-check's counter read it. `stalemate_loses` stays too.
- **Attack sets do not change, nor does their filter** (`capture_is_a_real_threat`: Atomic's
  adjacent kings). The jump lock (`continuing`) does not change.
- **The old keys `no_opponent_check`, `mandatory_capture`, `liberty_capture` (with its
  `allow_suicide`) and `forbid_repeated_positions` are refused with `OldForm`** and a message
  naming what replaced each, as slice 2's old forms are (`specs/game/old_forms.rs`).
- **Speed:** the timing run (`cargo run --release -p modern-chess-engine --example speed`) stays
  within 5% of `main` on chess, Shogi and Go, with identical move counts. `ATTACKED` must be
  lazy (stop at the first attacker that reaches a place), never building every threatened
  square of every opponent per way.
- **A spec that is wrong is refused when read, never read as "false".**
- Rust style of the crate: `cargo fmt`, `cargo clippy -p modern-chess-engine --all-targets` clean.
  Test names state behaviour (`snake_case_sentence`), as the existing ones do.

## Review Focus

Failure modes the design implies and no task would otherwise pin; each has its test in the task
named.

1. **A position with no king** (a hand-built test position) must not be filtered by the check
   rule: neither closing test can fail. (Task 5, `check_rule_ignores_a_side_with_no_king`.)
2. **Taking the enemy king while attacked** is legal, and in four-player chess taking any one
   enemy king is. (Task 5.)
3. **A Go move that captures first and then has a liberty is legal;** one that fills its own
   last liberty without capturing is refused. (Task 7.)
4. **A Go pass is never refused for repeating a position,** while a stone move that recreates
   one is. (Task 7.)
5. **A forced capture in the middle of a multi-jump** still limits the turn to the jumping
   piece (the jump lock is untouched), and a game with a forced capture offers no pass or drop
   while a capture exists. (Task 6.)

---

### Task 1: The tests `REMOVED`, `REPEATS` and `ATTACKED by MOVER`

**Files:**
- Modify: `crates/engine/src/specs/game/test_block.rs` (`TestSpec::Removed`, `Repeats`; `Attacked.by`)
- Modify: `crates/engine/src/specs/game/game.rs` (`validate_test`: accept `by` of `ENEMY` or `MOVER`)
- Modify: `crates/engine/src/logic/blueprint/test_block/{mod,build,judge}.rs`
- Modify: `crates/engine/src/logic/game/ways/run.rs` (`Run` records what a way removed; `Runner::context` passes it)
- Modify: `crates/engine/src/logic/game/liberties.rs` (turn `recreates_a_position` into `pub(crate) fn position_recurs(&self, pieces: &HashMap<Position, Piece>) -> bool`, which the later `settle` still calls)
- Test: `crates/engine/src/tests/every_move_blocks/mod.rs` (new; register it in `src/tests/mod.rs`)

**Interfaces:**
- Produces:
  - `Test::Removed(Filter)`, `Test::Repeats`, `Test::Attacked { region: Region, by: Attacker }` with
    `enum Attacker { Enemy, Mover }`.
  - `TestContext` gains `removed: &'a [Piece]` (the pieces the way has taken off the board so
    far, as they stood).
  - `Game::position_recurs`, as above.

- [ ] **Step 1: Write the failing tests** in `every_move_blocks/mod.rs`, on small inline specs
  built the way `tests/action_blocks/mod.rs` builds its (`small_spec`, `parse_game_spec_str`).
  - `removed_holds_after_a_way_takes_a_matching_piece`: a PROBE moves onto an enemy PAWN with a
    closing test `REMOVED where {owner: ENEMY, codes: [PAWN]}`; the way exists on the capture
    and does not exist on a quiet move.
  - `removed_counts_a_piece_taken_by_remove_move_and_place`: one spec per effect.
  - `removed_does_not_hold_for_an_ally_filter_when_an_enemy_was_taken`.
  - `repeats_holds_when_the_position_was_recorded_after_an_earlier_turn`: set
    `game.state.position_hashes` to the key of the position a move would create (use
    `Game::position_key_for` as `draw.rs` does), and a closing `REPEATS` test then removes that
    way and no other.
  - `repeats_is_false_in_a_game_that_has_recorded_nothing`.
  - `attacked_by_mover_looks_at_the_movers_own_attacks`: with a closing
    test `ATTACKED in AT(TARGET) by MOVER`, a move to a square the mover's other piece attacks
    passes, and one to an unattacked square does not.
  - Refusals when the spec is read: `by` other than `ENEMY`/`MOVER`; a `REMOVED` without `where`;
    `REPEATS` with a field; `REPEATS` in a spec with `jump_enabled` (the jump lock re-works ways
    mid-turn).

- [ ] **Step 2: Run them to see them fail.**
  Run: `cargo test -p modern-chess-engine every_move_blocks`
  Expected: compile errors or test failures naming the missing variants.

- [ ] **Step 3: Implement** the spec variants, their checks in `validate_test`, the compiled
  `Test` variants in `build.rs`, and `judge`:
  - `Removed(filter)` is true when some piece in `ctx.removed` passes `matches(filter, piece,
    ctx.player)`.
  - `Repeats` calls `ctx.game.position_recurs(ctx.pieces)` (the pieces as the steps so far leave
    them; the function clones, ticks timed flags, and builds the key for the player after the
    current one, exactly as `recreates_a_position` does now).
  - `Attacked` with `Mover` asks the mover's attack set, with `Enemy` the opponents'. (The lazy
    rewrite is Task 3; keep `attacked_by_pieces` here.)
  In `run.rs`, `Run.removed: Vec<Piece>` is pushed in `Runner::remove` (the piece taken, before
  the clear); `reads_pieces` returns true for `Repeats`, and `Removed` reads no pieces.

- [ ] **Step 4: Run the new tests, then the whole engine suite.**
  Run: `cargo test -p modern-chess-engine`
  Expected: all pass (the old forms still work).

- [ ] **Step 5: Commit** `feat(engine): the tests REMOVED, REPEATS and ATTACKED by MOVER`.

### Task 2: Regions `BOARD where`, `GROUP`, `AROUND` of a region, and the filter `empty`

**Files:**
- Modify: `crates/engine/src/specs/game/test_block.rs` (`RegionSpec::Board { r#where }`, `Group`, `AroundOf`; `FilterSpec.empty`)
- Modify: `crates/engine/src/specs/game/game.rs` (`validate_region`, `validate_filter`)
- Modify: `crates/engine/src/logic/blueprint/test_block/{mod,build,judge}.rs`
- Test: `crates/engine/src/tests/every_move_blocks/mod.rs`

**Interfaces:**
- Consumes: Task 1's module and test style.
- Produces: `Region::Board { filter: Option<Filter> }`, `Region::Group { of: Place, through: Through, filter: Filter }`, `Region::AroundRegion { of: Box<Region>, through: Through }`, and `Filter.empty: bool`. `AROUND`'s spec keeps its place form; the region form is chosen by the type of `of` (a place object has `place`, a region object has `region`).

- [ ] **Step 1: Write the failing tests.**
  - `board_with_a_filter_lists_only_the_places_of_matching_pieces`: `ATTACKED in BOARD where {owner: ALLY, codes: [KING]}` is judged only where an ally king stands (an attacked ally king passes; an attacked ally pawn does not).
  - `board_without_a_filter_is_every_place` (slice 2's behaviour kept).
  - `a_group_is_the_pieces_joined_through_orthogonal_neighbours_of_the_same_owner`: a 3-stone chain plus a separate stone and an enemy stone; `COUNT in GROUP(of AT(START)) ` equals 3.
  - `a_group_is_empty_where_the_place_holds_no_matching_piece`; `a_group_ignores_pieces_whose_code_the_filter_does_not_take`.
  - `around_a_region_is_the_neighbours_of_its_places_not_in_it_without_duplicates`.
  - `count_with_empty_counts_empty_places`: the empty neighbours of a group, `min: 0, max: 0` is true for a boxed-in group and false when one neighbour is free.
  - `a_neighbour_holding_a_piece_of_another_code_is_not_a_liberty`.
  - `a_group_and_around_work_through_a_maps_label` (a map board from `tests/map_spec`'s spec).
  - Refusals when read: `empty` with another filter field; `empty` in `GROUP`'s `where`;
    `GROUP` with no `of`; an unknown `through` label; `GROUP`/`AROUND` of a region on a board
    that lacks the label.

- [ ] **Step 2: Run to see them fail.** `cargo test -p modern-chess-engine every_move_blocks` — Expected: failures on the new names.

- [ ] **Step 3: Implement.** `judge.rs` `places()`: `Board` with a filter keeps places whose
  piece (in `ctx.pieces`) passes `matches`, or, for `empty`, places with no piece. `Group` walks
  neighbours with `Through` from the piece at `of` (a stack and a seen set, as `group_at` in
  `liberties.rs` does) keeping same-owner pieces that pass the filter; `AroundRegion` unions the
  neighbours of each place of `of`, drops those in `of`, deduplicates keeping order. `Count`
  with `filter.empty` counts the places of the region that hold no piece (`min` default 1).
  `matches` stays for pieces; `empty` is read only by `Count` and `Board`.

- [ ] **Step 4: Run the tests, then the whole engine suite.** Expected: all pass.

- [ ] **Step 5: Commit** `feat(engine): BOARD where, GROUP, AROUND of a region and the empty filter`.

### Task 3: A lazy `ATTACKED` that keeps the real-threat filter

**Files:**
- Modify: `crates/engine/src/logic/blueprint/test_block/judge.rs` (`attacked`)
- Modify: `crates/engine/src/logic/game/attack.rs` (one shared helper; `leader_in_check_for_pieces` uses it)
- Test: `crates/engine/src/tests/every_move_blocks/mod.rs`, and `src/tests/attack_map`, `not_attacked`, `atomic` stay green unchanged

**Interfaces:**
- Produces: `Game::reaches(&self, attacker: &str, pieces: &HashMap<Position, Piece>, places: &[&Position]) -> bool`
  in `attack.rs`: true when some piece of `attacker` has a threat on one of `places` that
  `capture_is_a_real_threat` accepts; it stops at the first and builds no set of all threats.
  `judge::attacked` calls it for each attacker (`ENEMY`: every other player; `MOVER`: the mover);
  `leader_in_check_for_pieces` calls it for its target places, keeping its "no leader on the
  board" answer. `capture_is_a_real_threat` becomes `pub(crate)`.

- [ ] **Step 1: Write the failing tests.**
  - `atomic_kings_may_stand_side_by_side_under_attacked`: in the Atomic spec, an enemy king
    adjacent to the mover's king is not an attack on it, judged by an `ATTACKED in BOARD where
    {owner: ALLY, codes: [KING]} by ENEMY` test (today's `leader_in_check_for_pieces` agrees).
  - `attacked_agrees_with_leader_in_check_on_every_catalogue_start_and_after_each_first_move`:
    for every catalogue spec with a non-empty `leader`, at the start and after the first way of
    each first-ply move, `ATTACKED in BOARD where ally leader codes by ENEMY` equals
    `game.leader_in_check()`. (This pins the lazy rewrite to the old answer.)
  - Laziness is proved by the speed gate in Task 9, not by a test.

- [ ] **Step 2: Run to see them fail.** Expected: the Atomic test fails (the eager `attacked`
  has no filter).

- [ ] **Step 3: Implement `Game::reaches`** (signature above) from the inner loop of
  `leader_in_check_for_pieces`; make `judge::attacked` and `leader_in_check_for_pieces` use it.
  Castling's `ATTACKED in PATH` goes through the same code; its results must not change (the
  castling tests are the check).

- [ ] **Step 4: Run the whole engine suite, then `catalogue_counts`.**
  Run: `cargo test -p modern-chess-engine`
  Expected: all pass, move counts identical.

- [ ] **Step 5: Commit** `refactor(engine): ATTACKED stops at the first attacker and keeps the real-threat filter`.

### Task 4: `every_action` and `must`

**Files:**
- Create: `crates/engine/src/specs/game/rule_steps.rs` (`EveryActionSpec { r#for: Vec<String>, r#do: Vec<DoSpec> }`)
- Modify: `crates/engine/src/specs/game/game.rs` (`GameSpec.every_action: Vec<EveryActionSpec>`, `must: Vec<TestSpec>`; `validate_every_action`, `validate_must`), `specs/game/mod.rs`, `specs/game/tests.rs` (the struct literal)
- Modify: `crates/engine/src/logic/blueprint/steps/build.rs` (append each applicable entry's compiled steps to each action's steps)
- Modify: `crates/engine/src/logic/game/mod.rs` (`Rules.must: Vec<Test>`; `removes_beyond_target` is computed over the appended steps too)
- Modify: `crates/engine/src/logic/game/ways/mod.rs` (`compute_ways` applies `must`)
- Test: `crates/engine/src/tests/every_move_blocks/mod.rs`

**Interfaces:**
- Consumes: Tasks 1–3's tests and regions.
- Produces: the spec fields above and `Game::narrow_by_must(&self, ways: &mut Vec<Way>)` in `ways/mod.rs`. A `must` test is judged on a way as it ended: its pieces are `simulate_from(&state.pieces, &way.changes)`, built only when `must` is non-empty, and `START`/`TARGET` are the way's first and last place answers. Give `Way` a `removed: Vec<Piece>` (kept from `Run`) so `REMOVED` can be judged without re-running the way.

- [ ] **Step 1: Write the failing tests.**
  - `an_every_action_test_removes_ways_in_every_action`: a spec with a MOVE and a PLACE action and one `every_action` entry with a test that fails on a particular target; both actions lose their ways to it.
  - `for_limits_an_entry_to_the_actions_it_names`.
  - `entries_run_in_the_order_written_after_the_actions_own_steps` (an entry sees the position the action's effects left; two entries: the second sees the first's effect).
  - `every_action_steps_run_after_a_moves_own_steps`: a piece move with a promotion, a closing test that reads the promoted code.
  - `must_keeps_only_the_ways_that_pass_when_some_way_does`; `must_changes_nothing_when_no_way_passes`; `must_entries_narrow_in_order`.
  - `must_removes_drops_and_passes_while_a_capture_exists` (an action list with MOVE, DROP and PASS).
  - Refusals when read: a `for` naming no action; a `choose` in `do`; a `must` entry that is not
    a test; unknown fields on an entry; `REPEATS` in `every_action` or `must` with `jump_enabled`
    (Task 1's refusal now also covers these places).

- [ ] **Step 2: Run to see them fail.**

- [ ] **Step 3: Implement.** Compile `every_action` in `steps_by_player` so each action's per-player
  step list ends with the entries that apply (those with no `for`, or naming that action), in
  order. `compute_ways` order: `run_actions` → (settle, until Task 7) → `must` narrowing → the
  jump lock; the old leader and `mandatory_capture` filters stay until their tasks.
  `piece_move_ways` does not append `every_action` (it judges a single threat, and must not
  recurse into check).

- [ ] **Step 4: Run the new tests, then the whole engine suite.** Expected: all pass.

- [ ] **Step 5: Commit** `feat(engine): every_action and must`.

### Task 5: Check, Atomic's own king and Racing Kings' opponent check as blocks

**Files:**
- Modify: `crates/engine/specs/*.json` — every spec with a non-empty `leader` (19: `4player_chess`, `atomic`, `capablanca`, `chaturanga`, `courier`, `crazyhouse`, `glinski`, `gothic`, `grand_chess`, `janggi_cannon`, `king_of_the_hill`, `mini_xiangqi`, `racing_kings`, `shatranj`, `shogi`, `solidarity`, `three_check`, `xiangqi`, `chess`), plus every book board and spec inside a test that names a `leader` and relies on check
- Modify: `crates/engine/src/logic/game/legal.rs` (delete `move_is_safe`, `retain_leader_safe`, `has_own_leader`, `leaves_leader_safe`, `retain_no_opponent_check`, `checks_an_opponent`; keep `any_legal_moves`), `ways/mod.rs`, `logic/game/mod.rs` (`no_opponent_check`), `specs/game/game.rs`, `specs/game/old_forms.rs`
- Modify: `crates/engine/src/tests/support/` (a `CHECK_RULE` helper that returns the closing block for given codes, for tests that build specs by hand), and the tests under `leader_safety`, `not_attacked`, `atomic`, `three_d`, `game_over`, `ways` as needed
- Create (uncommitted, throwaway): `.superpowers/sdd/2026-11-22-every-move-rules/tools/convert_check.py`
- Test: `crates/engine/src/tests/every_move_rules/mod.rs` (new)

**Interfaces:**
- Consumes: Tasks 1–4.
- Produces: the check rule as one block per spec, written as in the design's "Check" section, with `codes` equal to that spec's `leader`:
  `every_action: [ { "do": [ NOT(REMOVED where owner ALLY, codes LEADER), ANY[ NOT(ATTACKED in BOARD where owner ALLY, codes LEADER by ENEMY), REMOVED where owner ENEMY, codes LEADER ] ] } ]`.
  Racing Kings adds a second closing test, `NOT ATTACKED in BOARD where owner ENEMY, codes [KING] by MOVER`.

- [ ] **Step 1: Write the failing tests** in `every_move_rules/mod.rs`, with the spec file `chess.json` (rewritten in Step 3, so write them against the new spelling and watch them fail now):
  - `a_pinned_piece_may_not_leave_the_line`; `the_king_may_not_step_into_attack`; `a_move_that_answers_a_check_is_offered_and_others_are_not`.
  - `taking_the_enemy_king_is_allowed_even_if_ones_own_king_is_attacked`.
  - `check_rule_ignores_a_side_with_no_king` (Review Focus 1): a position with only the mover's rook and an enemy king, no ally king; every rook move is offered.
  - `four_player_check_filters_like_before` and `taking_any_one_enemy_king_in_four_player_is_enough` (Review Focus 2) on `4player_chess.json`.
  - `atomic_a_capture_that_blows_up_ones_own_king_is_refused` and `atomic_kings_may_stand_side_by_side`.
  - `racing_kings_a_move_that_checks_the_opponent_is_refused` and `racing_kings_a_move_that_does_not_is_offered`.
  - `the_old_keys_no_opponent_check_is_refused_with_old_form` (message names `every_action`).
  - `catalogue_counts` stays as the parity proof (no new test).

- [ ] **Step 2: Run to see them fail.** `cargo test -p modern-chess-engine every_move_rules`.

- [ ] **Step 3: Convert and delete.** Write `convert_check.py` (reads each spec's `leader`, adds the
  `every_action` block after `actions`, keeps the file's formatting style, adds the Racing Kings
  test; the same for `apps/docs/src/boards/*.json`, `apps/docs/examples/*.json`,
  `crates/game/tests/fixtures/*.json` and spec files under `crates/engine/src/tests` that name a
  `leader` and need check — inline specs get `CHECK_RULE`). Then delete the code listed above and
  turn `no_opponent_check` into a refused field. `leader` itself stays in specs and in `Rules`.

- [ ] **Step 4: Run the whole engine suite, then the session, wasm and game crates.**
  Run: `cargo test -p modern-chess-engine && cargo test -p modern-chess-session -p modern-chess-wasm -p modern-chess-game`
  Expected: all pass, `catalogue_counts` unchanged.

- [ ] **Step 5: Commit** `feat(engine)!: check and the opponent-check rule are written as blocks`.
  Note the commit as breaking.

### Task 6: Forced captures as `must`

**Files:**
- Modify: `crates/engine/specs/checkers.json`, `apps/docs/src/boards/checkers.json` (and any inline spec with `mandatory_capture`)
- Modify: `crates/engine/src/logic/game/ways/mod.rs` (delete the `mandatory_capture` branch), `logic/game/mod.rs`, `specs/game/game.rs`, `specs/game/old_forms.rs`
- Modify: `crates/engine/src/tests/checkers/mod.rs`, `rules_shared`, `map_play`, `ways` where they name the key
- Test: `crates/engine/src/tests/every_move_rules/mod.rs`

**Interfaces:** the rewrite is `"must": [ { "test": "REMOVED", "where": { "owner": "ENEMY" } } ]`.

- [ ] **Step 1: Write the failing tests** (Review Focus 5):
  - `a_capture_exists_so_only_captures_are_offered` (Checkers).
  - `a_man_in_the_middle_of_a_multi_jump_may_only_keep_jumping`.
  - `no_capture_means_quiet_moves_are_offered`.
  - `a_game_with_must_and_a_pass_action_offers_no_pass_while_a_capture_exists`.
  - `mandatory_capture_is_refused_with_old_form` (message names `must`).

- [ ] **Step 2: Run to see them fail.**
- [ ] **Step 3: Rewrite the specs, delete the branch, refuse the old key.**
- [ ] **Step 4: Run the whole engine suite.** Expected: all pass, `catalogue_counts` identical.
- [ ] **Step 5: Commit** `feat(engine)!: forced captures are a must entry`.

### Task 7: Liberty capture and repeated positions in Go as blocks

**Files:**
- Modify: `crates/engine/specs/go.json`, `go_13.json`, `go_19.json` (keep their compact formatting: the Go tests replace text in them), `apps/docs/src/boards/go.json`
- Delete: `crates/engine/src/logic/game/liberties.rs` (`settle`, `settle_changes`, `settle_liberties`, `group_at`, `orthogonal_neighbours` if nothing else reads it), `crates/engine/src/specs/game/liberty_capture.rs`
- Modify: `logic/game/ways/mod.rs` (drop the `settle` filter), `logic/game/mod.rs`, `specs/game/game.rs`, `specs/game/old_forms.rs`, `specs/game/tests.rs`, `tests/go/mod.rs`, `tests/rules_shared`
- Test: `crates/engine/src/tests/go/mod.rs`

**Interfaces:** the `STONE` action's rule, as an `every_action` entry with `"for": ["STONE"]` (so a pass is never refused for repeating):
1. `FOR_EACH` in `AROUND(of TARGET, orthogonal)`: `if` `ALL[ COUNT in AT(EACH) where {owner: ENEMY, codes: [STONE]}, COUNT{min 0, max 0} in AROUND(of GROUP(EACH, orthogonal, codes [STONE])) where {empty: true} ]` then `FOR_EACH` in `GROUP(EACH, orthogonal, codes [STONE])` do `REMOVE at EACH`.
2. test: `COUNT{min 1} in AROUND(of GROUP(TARGET, orthogonal, codes [STONE])) where {empty: true}` (suicide refused).
3. test: `NOT REPEATS`.
The spec for suicide-allowed (used in tests) replaces step 2 with an `if` that removes the group of `TARGET` when it has no liberty.

- [ ] **Step 1: Write the failing tests** in `tests/go/mod.rs` (respell the existing capture, suicide and ko tests against the rewritten specs; add):
  - `a_move_that_captures_first_then_has_a_liberty_is_legal`; `filling_ones_own_last_liberty_without_capturing_is_refused` (Review Focus 3).
  - `a_stone_move_that_recreates_a_position_is_refused_but_a_pass_is_not` (Review Focus 4).
  - `suicide_is_allowed_when_the_spec_removes_the_group_instead_of_refusing`.
  - `a_go_capture_carries_the_capture_mark_in_the_history` (the named behaviour change).
  - `liberty_capture_is_refused_with_old_form` and `forbid_repeated_positions_is_refused_with_old_form`.

- [ ] **Step 2: Run to see them fail.**
- [ ] **Step 3: Rewrite the three Go specs and the book board, delete the engine code, refuse the old keys.** The set of stones removed must equal today's (assert it in the existing tests); their order inside the way's changes may differ — respell any test that fixes it.
- [ ] **Step 4: Run the whole engine suite and the Go-related game/session tests.** Expected: all pass, Go's `catalogue_counts` identical.
- [ ] **Step 5: Commit** `feat(engine)!: liberty capture and ko are written as blocks`.

### Task 8: Docs, the spec reference and the book

**Files:**
- Modify: `crates/engine/specs/SPEC_FORMAT.md` (document `every_action`, `must`, the new tests/regions/filter; drop the removed forms; note that `leader` filters nothing and is read by the outcome until slice 4)
- Modify: `apps/docs/src/09-forced-captures.md`, `10-placing-stones.md`, `05-winning-and-drawing.md`, `11-design-your-own.md`, `known-limitations.md`, `reference.md`; `apps/docs/boards.derived.json` and `scripts/derive-boards.mjs` if a recipe names a removed key
- Modify: `docs/superpowers/specs/2026-11-22-rule-blocks-design.md` (move slice 3's carried findings: the two attack-set findings go to slice 5; add "slice 3 built" to the route)

- [ ] **Step 1:** Rewrite the chapters' examples to the blocks (Checkers' forced capture and Go's stones are the main ones); keep the book's voice and its "try it" boards working.
- [ ] **Step 2: Build and test the book boards.**
  Run: `apps/docs/build.sh && cargo test -p modern-chess-wasm --test boards && node --test apps/docs/scripts/derive-boards.test.mjs && node apps/docs/theme/test/smoke.mjs`
  Expected: all pass.
- [ ] **Step 3: Commit** `docs: the spec reference and the book teach every_action and must`.

### Task 9: The proof

- [ ] **Step 1: Move counts.** `cargo test -p modern-chess-engine catalogue_counts` — Expected: pass, identical counts for every game.
- [ ] **Step 2: Speed.** On this branch and on `main`, `cargo run --release -p modern-chess-engine --example speed` (same machine, back to back, twice).
  Expected: identical `moves=` per game; `median_ms` for chess, shogi and go within 5% of main. If chess or Go exceed it, find the cost (candidates: `Run.removed` clones, `materialise` per way, `Game::reaches`) and fix before going on.
- [ ] **Step 3: Everything.** `cargo test --workspace` (about 8 minutes in debug; run it once, in the background) and `pnpm --dir apps/web test`. Expected: all pass.
- [ ] **Step 4: In a browser** (`apps/web` with the game service, or the book): chess check and checkmate; Racing Kings; Checkers' forced jump; a Go capture, a refused suicide and a refused ko. Note anything that differs from before.
- [ ] **Step 5: Ledger the rulings and the deferred minors; finish with the final whole-branch review as the executing skill says.**
