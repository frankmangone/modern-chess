# Ends and counters Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `win_conditions`, `draw_conditions`, `leader` and `stalemate_loses` with `end_checks`, counters and a score, built from blocks.

**Architecture:** Additive first (new tests, effects, `end_checks`, `score` run beside the old forms), then every spec and test is respelled, then the old forms and their code are deleted and their keys refused with `OldForm`. End checks run in `Game::check_game_over` after the turn passes; counters are per-player numbers in the state, changed by `ADD`/`RESET` effects recorded on the `Way`.

**Tech Stack:** Rust (crate `modern-chess-engine`, lib `modern_chess`), serde JSON specs, the mdBook in `apps/docs`.

**Spec:** `docs/superpowers/specs/2026-11-23-ends-and-counters-design.md` (read it first; it fixes every block's name, fields and checks).

## Global Constraints

- Move counts per game identical (`catalogue_counts`); only the two named behaviour changes differ (Atomic ends on a blown-up king; Pong hau k'i draws after 12 plies).
- Timing run within 5% of the slice 3 branch on chess, Shogi and Go.
- A test, counter, outcome or `as` the engine does not know is refused when the spec is read.
- New blocks follow the existing style: spec enums with `deny_unknown_fields`, the checker in `specs/game/game.rs`, compiled types in `logic/blueprint/test_block`, judged in `judge.rs`.
- Commits: conventional, end with `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. No push.

## Review Focus

- An end check that reads `STUCK` must not make the ways be computed twice per turn.
- `REPEATS` in an end check must not tick flags or hand the turn on (the position is already after the turn).
- A four-player `LOSE` must leave the remaining players' turn order and cursor valid.
- Counters survive save and restore; a state without them restores with zeros.
- `SOME` nested in `SOME` must see the inner `EACH` and compute the inner region from the outer one.

### Task 1: New tests and filter fields

**Files:** `specs/game/test_block.rs`, `specs/game/game.rs` (checker), `logic/blueprint/test_block/{mod,build,judge}.rs`, `logic/game/ways/{mod,run}.rs` (`reads_pieces`, `Way.moved`), `logic/game/state/mod.rs` (counters), `logic/game/mod.rs`, `logic/game/attack.rs` (`keeping`), tests under `tests/end_blocks/`.

**Interfaces:**
- Produces: `TestSpec::{Some{r#in,that}, Passes{in_a_row}, Counter{name,min,max}, Moved{r#where}, Repeats{times}}`; `FilterSpec.except: Vec<String>`; `TestSpec::Attacked.keeping: Option<FilterSpec>`; `GameSpec.counters: Vec<String>`; `GameState.counters: HashMap<String /*player*/, HashMap<String /*counter*/, i64>>`; `Test::{Some{region,that},Passes(u16),Counter{name,min,max},Moved(Filter),Repeats{times}}`; `Filter.except`; `TestContext.{moved: &[Piece], settled: bool, stuck: bool}`; `Way.moved: Vec<Piece>`.

- [ ] Write failing tests (one per block, each refusal): `some_holds_when_a_place_passes`, `some_nested_sees_the_inner_each`, `passes_counts_trailing_passes`, `counter_reads_the_players_counter`, `moved_sees_a_pawn_push_not_a_capture_of_it`, `repeats_times_counts_recorded_positions`, `except_filters_codes`, `attacked_keeping_ignores_a_capture_that_loses_the_piece`; refusals: undeclared counter, `STUCK` outside an end check (Task 3), `in_a_row` 0, `times` 0, `except` with `empty`, `keeping` with `owner`.
- [ ] Run: expect compile errors / failures for the missing blocks.
- [ ] Implement the spec enums, the checker, the compiled tests, judging. `SOME` binds `EACH` per place of the region taken before it runs. `MOVED` reads `ctx.moved`; the runner records the piece as it stood in `move_piece` (board moves only). `REPEATS` with `settled` judges the position as it stands (no tick, no hand-on) and counts the recorded keys equal to it. `keeping` replaces `owns_a_leader` in `capture_is_a_real_threat`; `Game::reaches` takes `keeping: Option<&Filter>`.
- [ ] Run `cargo test -p modern-chess-engine`: green. Commit.

### Task 2: Counter effects

**Files:** `specs/game/action.rs` (`EffectSpec::{Add,Reset}`), `specs/game/game.rs` (checker, `counters`), `logic/blueprint/steps/{mod,build}.rs`, `logic/game/ways/{mod,run}.rs` (`Way.counts`), `logic/game/handlers/choose.rs` (apply on play), tests `tests/counters/`.

**Interfaces:** Produces `EffectSpec::Add{counter,by,to}`, `EffectSpec::Reset{counter,to}`; `Way.counts: Vec<CountChange>` where `CountChange{counter:String, to_all:bool, reset:bool, by:i64}`; applied by `Game::play_way`.

- [ ] Failing tests: `add_changes_the_movers_counter_when_the_way_is_played`, `add_to_all_changes_every_player`, `reset_sets_zero`, `a_way_not_played_changes_nothing`, `a_counter_test_in_a_way_reads_the_counter_as_the_turn_began`, refusals (undeclared counter, `by` 0, unknown `to`), `counters_survive_a_save_and_restore`, `a_state_without_counters_restores_with_zeros`.
- [ ] Implement; `Way.counts` is cloned with the way; `FOR_EACH` bodies may hold them. Run suite; commit.

### Task 3: `end_checks`, `score`, outcomes

**Files:** `specs/game/{end_check.rs (new),game.rs,mod.rs}`, `logic/game/{ends.rs (new, replaces win.rs/draw.rs checks),mod.rs,scoring.rs}`, `session/src/session.rs` (`scores()`), tests `tests/end_checks/`.

**Interfaces:** Produces `EndCheckSpec{as: Option<String>, when: TestSpec, then: OutcomeSpec}`, `ScoreTermSpec::{Pieces,Territory,Counter,Points}`, `GameSpec.{end_checks,score}`; `Game::scores() -> Option<BTreeMap<String,f64>>` (replaces `area_scores`); `Test::Stuck`.

- [ ] Failing tests: `win_ends_the_game_for_the_mover`, `lose_removes_the_next_player_and_ends_a_two_player_game`, `lose_in_four_players_keeps_the_rest_playing`, `draw_ends_with_no_winner`, `highest_gives_the_game_to_the_best_score_and_ties_draw`, `first_check_that_holds_wins_and_later_checks_do_not_run`, `stuck_computes_ways_once`, `as_next_reads_allies_as_the_player_to_move`, `repeats_in_an_end_check_counts_the_position_just_reached`, `scores_are_shown_live`, refusals (unknown outcome / `as`, `STUCK` not `as: NEXT`, `STUCK` outside an end check, a place name or `REMOVED`/`MOVED` in an end check, `HIGHEST` with no `score`, score term with `owner`).
- [ ] Implement `Game::check_game_over`: record the position; judge the end checks in order with `ways` computed lazily once; apply the outcome (reusing `eliminate_player`); otherwise start choosing with the same ways. Old win/draw/stalemate code still runs first when a spec still has the old keys.
- [ ] Suite green; commit.

### Task 4: Respell every spec and test

**Files:** `crates/engine/specs/*.json`, `crates/game/tests/fixtures/*.json`, `crates/session/tests/fixtures/bare.json`, `apps/docs/src/boards/*.json`, `apps/docs/examples/*.json`, specs inside tests (`tests/**`), `tools/convert_ends.py` (untracked).

- [ ] Conversion per `What each old form becomes` in the design: win types, draw conditions, checkmate/stalemate pair (or stalemate-loses), Three-check counter, fifty-move counter, Go's `score`, Atomic's king-gone check and `keeping`.
- [ ] `catalogue_counts` identical; each converted spec's existing tests unchanged in what they check, except the two named behaviour changes.
- [ ] Commit per group (catalogue, fixtures and book boards, inline test specs).

### Task 5: Delete the old forms

**Files:** `specs/game/{win_condition.rs,draw_conditions.rs,game.rs,old_forms.rs,tests.rs}`, `logic/game/{win.rs,draw.rs,connection.rs,scoring.rs,attack.rs,mod.rs,state/mod.rs,legal.rs}`, `shared/constants.rs`, tests of the old forms.

- [ ] Remove `win_conditions`, `draw_conditions`, `leader`, `stalemate_loses`, `players_with_configured_leader`, `leader_in_check*`, `WinConditionKind`, `check_counts`, `area_scores`, `connects`; refuse the four keys with `OldForm` naming `end_checks`.
- [ ] The position record leaves `moved` out unless the spec reads it anywhere (`Rules.moved_matters`); test: Pong hau k'i draws after 12 plies.
- [ ] Old-key tests: each of the four refused with a message naming `end_checks`. Suite green; commit.

### Task 6: Docs

**Files:** `specs/SPEC_FORMAT.md`, `apps/docs/src/05-winning-and-drawing.md` (rewritten around the blocks), chapters 02, 03, 08, 09, 10, 11, 12, `known-limitations.md`, the route doc (slice 4 marked built).

- [ ] Every board in the book plays under the new blocks (the book's board tests). Commit.

### Task 7: Final review and fixes

- [ ] Timing run vs slice 3 branch (alternating, release). Whole-branch review on opus; one fix pass, each fix RED→GREEN. Ledger, rulings list.
