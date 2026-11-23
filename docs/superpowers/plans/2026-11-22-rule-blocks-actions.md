# Rule blocks, slice 2: actions and effects — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every turn in every spec is an action made of choose / test / effect / if steps, and
one protocol (`choose` / `cancel`) runs from the engine to the board.

**Architecture:**
- **Spec types.** The new spec types (`ActionSpec`, `DoSpec`, `ChoiceSpec`, `EffectSpec`) compile
  once per spec into a step tree, with every place turned per player.
- **Ways.** One runner walks a position's steps into *ways*: the complete answer lists, each
  with the changes it makes. The engine's single `Choosing` phase filters the ways by the
  answers given so far.
- **The migration runs in stages, and every task ends green:**
  - The new blocks and the runner arrive beside the old forms. A spec without `actions` is run
    through a *legacy adapter* that turns today's effects into ways (Tasks 1–2).
  - The engine protocol switches to ways, while the session keeps its public API on top
    (Task 3).
  - Every spec is rewritten, and the old forms and the adapter are deleted (Tasks 4–5).
  - Each layer above the engine moves to the new protocol (Tasks 6–8). Then the docs (Task 9).

**Tech Stack:**
- Rust: the engine, session, wasm and game crates; serde; sqlx migrations.
- TypeScript/Solid in `apps/web`, the board element JS in `apps/docs/theme`, the mdBook book,
  and pnpm.

**Spec:** `docs/superpowers/specs/2026-11-22-actions-and-effects-design.md`. Read it whole,
including "Frank's answers". It builds on slice 1, `docs/superpowers/specs/2026-11-22-rule-blocks-design.md`.

## Global Constraints

- **JSON shapes exactly as in the design's tables.** Nothing else is accepted.
  - Steps are `{ "choose": KIND, ... }`, `{ "test": ... }` (slice 1), `{ "effect": KIND, ... }`
    or `{ "if": TEST, "then": [...], "else": [...] }`. A step carries exactly one of those four
    keys.
  - Choices are `PIECE` (`where`), `MOVE`, `PLACE` (`in`), `HAND` (`where`, `codes` only) and
    `CODE` (`as` required, `options`). Every choice takes an optional `as`.
  - Effects are:
    - `MOVE` (`from`, `to`), `REMOVE` (`at`), `PLACE` (`at`, `code`), `SET_OWNER` (`at`);
    - `SET_CODE` (`at`, `code`), `SET_FLAG` (`at`, `flag`, optional `turns`);
    - `FOR_EACH` (`in`, `do`), `END_TURN`.
  - A code is a string, or `{ "chosen": NAME }`.
  - Actions are `{ "name", "players", "do" }`.
  - The new region is `{ "region": "BOARD" }`, and the new place is `{ "place": "EACH" }`.
- **Steps run in order on the changing position.** A test or effect after an effect sees the
  position as the earlier steps left it. Tests before a move's first effect see the position
  before the turn.
- **`MOVE` replaces whatever stands on `to`.** `PLACE` onto a piece removes it first. Every
  removal stays visible in the way's changes, so the captured-material count, the hand and the
  history's capture mark all come from the changes (Frank's answer 2).
- **A capture** is a way that removes a piece of another player, by `REMOVE`, `MOVE` onto it or
  `PLACE` onto it. **A pass** is a way with no changes. Nothing anywhere matches the strings
  `"CAPTURE"`, `"MOVE"`, `"TRANSFORM"`, `"DROP"` or `"PASS"` once Task 5 lands.
- **Has moved.** A `MOVE` effect adds one move only to the piece the `PIECE` choice picked.
  `SET_CODE` and `PLACE` make new, unmoved pieces. A piece moved from the hand enters unmoved.
- **`END_TURN`** stops the jump continuation for that way.
- **Checkers follows standard English draughts**, written in blocks (Frank's answer 5):
  - a crowning jump is a capture, forced like any other;
  - it may end a multi-jump;
  - crowning ends the turn (`SET_CODE KING` then `END_TURN`).
- **Only options that can finish are offered.** An option needs a complete way that passes
  every rule applied to every move today: leader safety, liberty capture, no repeated
  positions, `no_opponent_check`, `mandatory_capture` and the jump lock.
- **Ways are worked out once per turn** (on a turn's start, and on restore). Answers only
  filter them. Two ways of one action with equal answers keep the first in spec order.
- **Strictness.** Every new block refuses unknown fields, and every refusal in the design's
  "Checked when a spec is read" is a `GameSpecError` in today's style.
- **Old forms are refused with `OldForm`** naming their replacement (from Task 5): a move's
  `actions`, `conditions`, `modifiers` and `side_effects`; `allow_pass`; `unlimited_supply`;
  `drop_conditions`.
- **Unchanged in this slice:** `threat_only`, `hand_enabled`, `demotes_to`, `jump_enabled`,
  `mandatory_capture`, `leader`, `no_opponent_check`, `liberty_capture`,
  `forbid_repeated_positions` and the win and draw conditions.
- **Grid speed.** `cargo run --release -p modern-chess-engine --example speed` stays within 5%
  of `main` on chess, Shogi and Go, run side by side with the median of three runs each. The
  speed run and `catalogue_counts` both count distinct (piece place or hand code, target)
  pairs, and their numbers stay identical to today's table.
- **Tests.** Every existing test keeps what it checks. Tests are respelled for the new
  protocol, never weakened.
- **Process.**
  - `make lint-panics` stays clean.
  - Conventional commits, each ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
  - Use pnpm for JS. Never push.
  - Leave ports 3001 and 3050 alone.
  - Don't touch the untracked `images/`, `pocs/`, `modern-chess.pen` or `.DS_Store`.

## Review Focus

1. **Clicking another of your pieces while its destinations are shown** chooses that piece
   instead. Clicking an empty square that answers nothing deselects. In Task 6, test
   `clicking_another_piece_while_a_move_is_open_chooses_it` and test
   `clicking_a_place_that_answers_nothing_cancels`, both in `crates/session/tests/play.rs`.
2. **While a promotion is asked:**
   - the board shows the pawn on the last rank;
   - `cancel` is refused;
   - the opponent's snapshot shows the pawn moved but no `choice`.

   Task 3 test `a_promotion_shows_the_moved_pawn_and_cannot_be_cancelled`. Task 7 test
   `the_choice_is_hidden_from_the_player_not_to_move`.
3. **Two placing actions on the same point** (the `STONE`/`DISC` fixture). The view lists both
   as narrowing actions (`plays: false`). Answering one first, or after the point, plays the
   right piece. Task 2 test `two_placing_actions_are_told_apart_by_name`. Task 8 keeps
   `with_two_supply_pieces_one_has_to_be_chosen_first`.
4. **Restoring a saved game mid-promotion or mid-jump** keeps the answers and the jump lock.
   Task 3 test `a_restored_game_keeps_its_answers_and_its_jump_lock`. Task 6 test
   `resume_keeps_an_open_promotion`.
5. **A malformed answer** is refused, and the game is left unchanged. Malformed means:
   - an unknown code;
   - a place that is not a node;
   - a hand piece not held;
   - an unknown action name;
   - a command with two answer keys, or none.

   Task 3 test `answers_that_are_not_options_change_nothing`. Task 7 test
   `a_choose_command_needs_exactly_one_answer`.

---

### Task 1: Action and step blocks: spec types, compiled tree, refusals

**Files:**
- Create:
  - `crates/engine/src/specs/game/action.rs`: `ActionSpec`, `DoSpec`, `ChoiceSpec`,
    `EffectSpec`, `CodeSpec`, `IfSpec`;
  - `crates/engine/src/logic/blueprint/steps/{mod.rs,build.rs}`: the compiled tree;
  - `crates/engine/src/tests/action_blocks/{mod.rs,spec.json}`.
- Modify:
  - `crates/engine/src/specs/game/piece/action.rs`: rename the old type to `LegacyActionSpec`;
  - `crates/engine/src/specs/game/piece/move.rs`: add `do`;
  - `crates/engine/src/specs/game/game.rs`: `actions`, plus validation;
  - `crates/engine/src/specs/game/test_block.rs`: `RegionSpec::Board`, the place `EACH`;
  - `crates/engine/src/logic/blueprint/test_block/{build,judge}.rs`: `BOARD` and `EACH`;
  - `crates/engine/src/logic/blueprint/{move_blueprint/mod.rs,move_blueprint/build.rs,piece_blueprint.rs}`;
  - `crates/engine/src/tests/mod.rs`.

**Interfaces:**
- **Produces (spec):**
  - `GameSpec.actions: Vec<ActionSpec>`. The default is empty, which means "legacy" until
    Task 5.
  - `ActionSpec { name: String, players: Vec<String>, r#do: Vec<DoSpec> }`.
  - `MoveSpec.r#do: Option<Vec<DoSpec>>`.
  - `enum DoSpec { Choose(ChoiceSpec), Test(TestSpec), Effect(EffectSpec), If(IfSpec) }`. It is
    told apart by which one of `choose`/`test`/`effect`/`if` is present, using a custom
    `Deserialize`.
  - `ChoiceSpec`, internally tagged on `"choose"`:
    - `Piece { r#where: FilterSpec, r#as }`;
    - `Move { r#as }`;
    - `Place { r#in: RegionSpec, r#as }`;
    - `Hand { r#where: Option<FilterSpec>, r#as }`;
    - `Code { r#as: String, options: Vec<String> }`.
  - `EffectSpec`, internally tagged on `"effect"`:
    - `Move { from: PlaceSpec, to: PlaceSpec }`, `Remove { at }`, `Place { at, code: CodeSpec }`;
    - `SetOwner { at }`, `SetCode { at, code }`, `SetFlag { at, flag: String, turns: Option<u8> }`;
    - `ForEach { r#in: RegionSpec, r#do: Vec<DoSpec> }`, `EndTurn`.
  - `CodeSpec`, untagged: `Code(String)` or `Chosen { chosen: String }`.
  - `IfSpec { r#if: TestSpec, then: Vec<DoSpec>, r#else: Vec<DoSpec> }`.
  - All of these deny unknown fields.
- **Produces (engine):**
  - `enum Step { Choose(Choice), Test(Test), Effect(EffectStep), If { test: Test, then: Vec<Step>, otherwise: Vec<Step> } }`.
  - `enum Choice { Piece(Filter), Move, Place(Region), Hand(Vec<String>), Code { name: String, options: Vec<String> } }`.
  - `enum EffectStep { Move { from: Place, to: Place }, Remove(Place), Put { at: Place, code: CodeRef }, SetOwner(Place), SetCode { at: Place, code: CodeRef }, SetFlag { at: Place, flag: String, turns: Option<u8> }, ForEach { region: Region, steps: Vec<Step> }, EndTurn }`.
  - `enum CodeRef { Code(String), Chosen(String) }`.
  - `Step::build(spec: &DoSpec, players: &[PlayerSpec], step: Option<&StepSpec>, labels: &mut Labels) -> Step`, built per player as `Test::build` is.
  - Today's `ActionBlueprint` (the occupancy-keyed one) is renamed `LegacyActionBlueprint`.
  - The new compiled action is `Action { name: String, players: Vec<String>, steps: HashMap<String, Vec<Step>> }`,
    with the steps keyed by player.
  - `MoveBlueprint.steps: Option<HashMap<String, Vec<Step>>>`.
  - `MoveBlueprint.threatens: bool`. It is false when a test before the move's first effect
    requires `TARGET` to be empty. That test is `NOT(COUNT in AT(TARGET))` with no `where`, or
    `COUNT in AT(TARGET)` with `min: 0, max: 0` and no `where`, found in the step list or in a
    top-level `ALL`.
  - `Rules.actions: Vec<Action>`.
  - `Rules.removes_beyond_target: bool`. It is true when some move or action has a `REMOVE`
    inside `FOR_EACH`, or a `REMOVE` at `START` with no offset.

- [ ] **Step 1: Write the failing tests** in `tests/action_blocks/mod.rs`, against
  `spec.json`. That spec is an 8×8 grid with WHITE and BLACK (180°), and every move in `do`
  form plus `actions`, following the design's examples. Tests:
  - `every_step_kind_parses` (one of each choice and effect, `if` with and without `else`,
    `FOR_EACH` and `END_TURN`);
  - `a_code_is_a_string_or_a_chosen_name`;
  - `board_region_and_each_place_parse`;
  - `a_misspelled_field_in_a_step_is_refused` (`"chose"`, and `"form"` inside a `MOVE` effect);
  - `a_step_with_two_kinds_or_none_is_refused` (`{ "test": …, "effect": … }`, and `{}`);
  - `refusals_when_the_spec_is_read`. Each is a `GameSpecError` whose text names the
    offending value:
    - `MOVE` with no earlier `PIECE`;
    - `TARGET` used before any choice sets it;
    - `EACH` outside `FOR_EACH`;
    - `{ "chosen": "promo" }` with no `CODE` named `promo`;
    - an unknown code in `PLACE`, `SET_CODE` or `CODE.options`;
    - a choice inside `FOR_EACH`;
    - duplicate action names;
    - an unknown player in `players`;
    - a spec that mixes `actions` with an old-form move, or old forms with a `do` move
      (transitional);
  - `start_after_a_hand_choice_is_allowed_in_tests` (slice 1's "START refused in a drop" no
    longer applies once a `HAND` choice sets it);
  - `a_move_that_requires_an_empty_target_does_not_threaten`, and
    `a_move_that_allows_an_enemy_target_threatens` (`threatens`);
  - `removes_beyond_target_is_noted_for_an_atomic_like_capture`.

- [ ] **Step 2: Run them to see them fail.**
  Run: `cargo test -p modern-chess-engine action_blocks`
  Expected: it fails to compile (no module, no `do` field), then every test FAILS against stubs.

- [ ] **Step 3: Add the spec types, `Step::build`, `threatens`, `removes_beyond_target` and the
  validation.**
  - Put the validation in `GameSpec::validate` as `validate_actions`.
  - Add one new error variant, `InvalidStep(String)`. Reuse existing variants where they fit
    (`UnknownPieceInRule`, `NotOnAMap`, `UnknownPlayer…`).
  - The placement check is static. It walks the steps in order, tracking which of `START`,
    `TARGET` and named codes are set. An `if`'s two branches each continue from the set as it
    was before the `if`, and what is set after it is what both branches set.

- [ ] **Step 4: Run the engine suite.**
  Run: `cargo test -p modern-chess-engine`
  Expected: all PASS. No catalogue spec uses the new forms yet.

- [ ] **Step 5: Commit:**
  `git commit -m "feat(engine): action and step blocks in the spec, compiled per player"`

---

### Task 2: Ways: the step runner, and the legacy adapter

**Files:**
- Create:
  - `crates/engine/src/logic/game/ways/mod.rs`: `Way`, `Answer`, and `Game::compute_ways`;
  - `crates/engine/src/logic/game/ways/run.rs`: runs steps into ways;
  - `crates/engine/src/logic/game/ways/legacy.rs`: the adapter for specs without `actions`;
  - `crates/engine/src/tests/ways/{mod.rs,spec.json,two_supplies.json}`.
- Modify:
  - `crates/engine/src/logic/blueprint/test_block/judge.rs`: `TestContext` gains `pieces`;
  - `crates/engine/src/logic/blueprint/move_blueprint/generate.rs`: expose the candidate
    targets of a move for the runner;
  - `crates/engine/src/logic/game/{liberties.rs,legal.rs,attack.rs}`: `settle`, the leader
    safety filter and the `no_opponent_check` filter take a `Way`; the Atomic gate reads
    `removes_beyond_target`;
  - `crates/engine/src/logic/mod.rs`.

**Interfaces:**
- **Consumes:** Task 1's `Step`, `Choice`, `EffectStep`, `CodeRef`, `Rules.actions` and
  `MoveBlueprint.steps`/`threatens`.
- **Produces:**
  - `#[serde(rename_all = "lowercase")] enum Answer { Place(Position), Code(String), Hand(String), Action(String) }`.
    It serializes as `{"place":"4,2"}`.
  - `struct Way`, with these fields:
    - `action: String`;
    - `answers: Vec<Answer>`: the place, code and hand answers in order. It never holds
      `Answer::Action`, because the action's name is in `action`;
    - `kinds: Vec<ChoiceKind>`: what each answer answered;
    - `changes: Vec<BoardChange>`;
    - `from_hand: Option<String>`;
    - `captures: bool`;
    - `ends_turn: bool`;
    - `lands: Option<Position>`: where the picked piece ended.
  - `enum ChoiceKind { Piece, Move, Place, Code, Hand }`. It serializes as `"PIECE"`,
    `"MOVE"`, `"PLACE"`, `"CODE"` or `"HAND"`.
  - `pub(crate) fn Game::compute_ways(&self) -> Vec<Way>`. It returns every complete way for
    the player to move, after `settle`, leader safety, `no_opponent_check`, `mandatory_capture`
    and the jump lock. Ways keep their action's name, so the caller can narrow by it.
  - `TestContext { game, pieces: &'a HashMap<Position, Piece>, player, start, target, each: Option<&'a Position> }`.
    `pieces` is the position being judged: `&game.state.pieces` until the first effect, then
    the position as the effects so far left it.
- **Legacy adapter** (it is deleted in Task 5). For a spec with no `actions`, it synthesizes
  these, keeping today's behaviour exactly:
  - **`MOVE`:** answers `[Place(piece), Place(target)]`, from today's `calculate_moves`.
    - `changes` comes from the effect's `board_changes`.
    - `captures` is `effect.action == "CAPTURE"`.
    - `ends_turn` is `effect.action == "TRANSFORM"`.
    - A `TRANSFORM` with options becomes one way per option. Each gets an extra `Code(option)`
      answer, and appends `set_piece(target, Piece::new(option, mover))`, but only when a piece
      still stands on `target`.
  - **`DROP`** (with `hand_enabled`): `[Hand(code), Place(square)]`, from
    `compute_drop_squares`, with `from_hand` set.
  - **One action per `unlimited_supply` code**, named after the code: `[Place(square)]`.
  - **`PASS`** when `allow_pass`: `[]`.

- [ ] **Step 1: Write the failing tests** in `tests/ways/mod.rs`. Use `spec.json` (new form)
  and `two_supplies.json` (two placing actions, `STONE` and `DISC`, both
  "choose `PLACE` in `BOARD`"). Each test calls `compute_ways` and checks answers and changes:
  - `a_piece_move_gives_one_way_per_reachable_place`;
  - `a_slide_stops_where_until_says`;
  - `move_onto_an_enemy_removes_it_and_is_a_capture`;
  - `place_puts_a_new_unmoved_piece_owned_by_the_mover`;
  - `set_owner_set_code_and_set_flag_change_the_piece`, including that `turns: 1` expires
    after the next turn, as today;
  - `for_each_runs_once_per_place_of_its_region_taken_before_it_runs`;
  - `if_runs_then_or_else`;
  - `a_test_after_an_effect_sees_the_changed_position`;
  - `a_code_choice_after_a_move_gives_one_way_per_option`;
  - `a_hand_choice_sets_start_and_move_from_it_takes_one_from_the_hand`;
  - `has_moved_counts_only_for_the_picked_piece` (castling: the king counts a move, the rook
    does not);
  - `set_code_makes_an_unmoved_piece`;
  - `end_turn_is_recorded_on_the_way`;
  - `place_then_remove_an_enemy_is_one_turn` (`PLACE` at the chosen place, then `REMOVE` at
    `TARGET+[1,0]`: a kind of turn today's engine cannot express);
  - `board_region_lists_every_place_on_a_map` (a `PLACE` choice in `BOARD` on a small map);
  - `only_options_that_can_finish_are_offered` (a move whose later test always fails gives no
    way);
  - `an_empty_action_is_a_pass_with_no_changes`;
  - `two_placing_actions_are_told_apart_by_name` (two ways with equal answers, from `STONE`
    and `DISC`);
  - `equal_answers_in_one_action_keep_the_first_in_spec_order`;
  - `legacy_ways_match_todays_moves_for_every_catalogue_spec`. For each catalogue spec at its
    start position, the set of (from, to) pairs from `compute_ways` equals today's
    `calculate_moves` destinations, plus today's drop squares.

- [ ] **Step 2: Run them to see them fail.**
  Run: `cargo test -p modern-chess-engine ways`
  Expected: compile failure, then FAIL against stubs.

- [ ] **Step 3: Implement `run.rs`, `legacy.rs`, `compute_ways`, and move `settle`, leader
  safety, `no_opponent_check` and the Atomic gate onto `Way`.**
  - A `MOVE` choice takes the candidate targets of each of the piece's moves from the existing
    generator: its `step`, `repeat`/`until`, `players` and `threat_only` exclusion. It then runs
    that move's steps for each target.
  - The tests before a move's first effect are judged on `&game.state.pieces`, without
    copying.
  - The position after an effect is materialised once per way, only when a later step reads
    it.
  - `capture_is_a_real_threat` simulates only when `rules.removes_beyond_target` is true.
  - The old `calculate_moves`, `calculate_drops`, `execute_*`, `transform` and `pass` stay
    working for now.

- [ ] **Step 4: Run the engine suite.**
  Run: `cargo test -p modern-chess-engine`
  Expected: all PASS.

- [ ] **Step 5: Commit:**
  `git commit -m "feat(engine): work out every way of taking an action, with a legacy adapter"`

---

### Task 3: The engine's one protocol: Choosing, Choose, Cancel

**Files:**
- Create:
  - `crates/engine/src/logic/game/handlers/{choose.rs,cancel.rs}`;
  - `crates/engine/src/logic/game/ways/choice.rs` (`Choice`);
  - `crates/engine/src/tests/support/mod.rs`, with test helpers.
- Delete: `crates/engine/src/logic/game/handlers/{calculate_moves,execute_move,transform,calculate_drops,execute_drop,pass}.rs`.
- Modify:
  - `crates/engine/src/logic/game/{phase.rs,transition.rs,transitions.rs,mod.rs,draw.rs,io.rs,legal.rs}`;
  - `crates/engine/src/logic/game/state/{mod.rs,move_record.rs}`;
  - `crates/engine/examples/speed.rs`;
  - `crates/engine/src/tests/catalogue_counts/mod.rs`;
  - every engine test that drives turns (about 43 files under `crates/engine/src/tests`);
  - `apps/cli/src/main.rs`;
  - `crates/session/src/{session.rs,view.rs}`, internals only. Session's public methods and
    view fields stay as they are.
  - `crates/game` only if it fails to compile.

**Interfaces:**
- **Consumes:** Task 2's `Way`, `Answer`, `ChoiceKind` and `compute_ways`.
- **Produces:**
  - `enum GamePhase { Choosing { chosen: Vec<Answer> }, GameOver { winner: Option<String> } }`.
  - `GameState.continuing: Option<Position>` (serialized), the piece that must keep capturing.
  - `GameState.ways: Option<Vec<Way>>` (`#[serde(skip)]`). It replaces `available_moves`, and
    is filled on a turn's start and by `restore`.
  - `enum GameTransition { Choose(Answer), Cancel }`.
  - `GameError::NotAnOption` and `GameError::CannotCancel`.
  - `pub fn Game::choice(&self) -> Choice`.
  - `struct Choice { chosen: Vec<Answer>, start: Option<Position>, places: Vec<PlaceOption>, codes: Vec<String>, hand: Vec<String>, actions: Vec<ActionOption> }`.
  - `struct PlaceOption { name: Position, r#for: ChoiceKind }`, serialized with `"for"`.
  - `struct ActionOption { name: String, plays: bool }`.
  - `pub fn Game::shown_pieces(&self) -> HashMap<Position, Piece>`. It is the board with the
    effects so far applied, while every remaining way agrees on them. Otherwise it is the board
    itself.
  - `MoveRecord { player, action: String, chosen: Vec<Answer>, piece_code: String, from: Position, to: Position, captures: bool, passed: bool }`.
    `from`/`to` are empty when there is none, as today. `promotion` is removed.
- **Rules of the protocol** (from the design's "The protocol"):
  - Answering filters the ways.
  - An action name is always a valid answer, and narrows to that action.
  - When the answers match exactly one complete way, it is played.
  - `places`/`codes`/`hand` list the next answers of the remaining ways.
  - `actions` lists:
    - the actions whose ways are complete with no further answer (`plays: true`);
    - when more than one action remains, every remaining action (`plays: false`);
    - a complete way's action beside longer ways sharing its answers (`plays: true`).
  - `Cancel` clears `chosen`. It is refused while `continuing` is set, and once any answer has
    made an effect showable (after a `MOVE`, during a promotion).
  - After a capturing way without `ends_turn`, under `jump_enabled`, if the piece at `lands`
    has capturing ways, the same player continues: `continuing = lands`, and `chosen` starts as
    `[Place(lands)]`.
  - The fifty-move rule resets on `captures` or a pawn code.
  - `consecutive_passes` counts records with `passed`.
- **Session shim (until Task 6):**
  - `select(p)` is `Cancel` (when allowed) then `Choose(Place(p))`.
  - `make_move(p)` and `promote(c)` are `Choose`.
  - `drop_piece(c, p)` answers `Action(c)` first if an action named `c` exists, otherwise
    `Hand(c)`, then `Place(p)`.
  - `pass()` is `Choose(Action("PASS"))`.
  - The old view fields are worked out from `choice()`:
    - `selected = start`;
    - `destinations` are the `MOVE` places;
    - `promotion` is `codes`, when not empty;
    - `drops` come from the ways starting `Hand`/supply action;
    - `can_pass` is whether a `plays` action is named `PASS`.
  - View `pieces` come from `shown_pieces`.

- [ ] **Step 1: Write the failing tests** in `crates/engine/src/tests/ways/protocol.rs`
  (add it to the module):
  - `choosing_a_piece_then_a_place_plays_the_move`;
  - `the_choice_lists_movable_pieces_then_destinations`;
  - `a_promotion_shows_the_moved_pawn_and_cannot_be_cancelled`;
  - `cancel_before_any_effect_clears_the_answers`;
  - `an_action_name_narrows_and_a_pass_plays_at_once`;
  - `a_shorter_complete_way_is_offered_beside_longer_ones`;
  - `the_jump_lock_starts_the_next_turn_with_the_piece_chosen_and_refuses_cancel`;
  - `end_turn_stops_the_jump_continuation`;
  - `answers_that_are_not_options_change_nothing` (unknown code, unknown node, hand piece not
    held, unknown action: each `NotAnOption`, with state equal before and after);
  - `a_restored_game_keeps_its_answers_and_its_jump_lock`;
  - `the_history_records_the_action_its_answers_and_captures`;
  - `the_fifty_move_rule_resets_on_a_capture_way`;
  - `two_passes_in_a_row_are_counted`.

- [ ] **Step 2: Run them to see them fail.**
  Run: `cargo test -p modern-chess-engine ways::protocol`
  Expected: compile failure, then FAIL.

- [ ] **Step 3: Implement the phase, transitions, `choice`, `shown_pieces`, the continuation and
  the history, and delete the old handlers.**
  - Add helpers to `tests/support`: `pick(&mut Game, &str)`, `go(&mut Game, &str)`,
    `promote(&mut Game, &str)`, `drop(&mut Game, &str, &str)`, `pass(&mut Game)`,
    `destinations(&Game) -> Vec<String>` and `phase_is_choosing(&Game)`.
  - Rewrite the engine tests' turn-driving through those helpers, so what each test asserts is
    unchanged.
  - **Walkers.** `catalogue_counts` and `speed.rs` walk `ways`. They count distinct
    (`answers[0]` place or hand code, target place) pairs, and recurse into the first way of
    each pair, which is today's "first promotion option". The count table stays as it is.
  - **CLI.** `apps/cli` prints `choice()` and reads one answer per prompt.
  - **Session.** Use the shim above. Session tests must pass unchanged.

- [ ] **Step 4: Run every Rust test, the lint, and the speed run.**
  - Run: `cargo test --workspace`. Expected: all PASS, including every session and game test,
    unchanged. The DB tests need `make db-up`; if it is not running, say so in the report.
  - Run: `make lint-panics`. Expected: clean.
  - Run: `cargo run --release -p modern-chess-engine --example speed`. Expected: counts
    identical to `main`. Record the times in the report.

- [ ] **Step 5: Commit:**
  `git commit -m "feat(engine)!: one protocol, choose and cancel over the ways of each action"`

---

### Task 4: Rewrite the catalogue specs in steps

**Files:**
- Modify: every `crates/engine/specs/*.json` (26 files), and
  `apps/docs/scripts/derive-boards.mjs` if a recipe relies on an old field.
- Test: `crates/engine/src/tests/{catalogue_counts,checkers,atomic,ataxx,shogi,castling}/…`.

**Interfaces:**
- **Consumes:** Task 1's spec format, and Task 3's protocol.
- **Each spec gets `actions` and every move in `do` form, as in the design's "How the
  catalogue reads after the rewrite" table.**
  - Chess-likes: `MOVE` (`PIECE` with `owner: ALLY`, then `MOVE`).
  - Shogi and Crazyhouse add `DROP` (`HAND`, then `PLACE` in `BOARD`, with the old
    `drop_conditions` as tests on `START`'s code, then `MOVE START→TARGET`).
  - Go and Hex: an action named after the stone's code (`STONE` in Go). It chooses `PLACE` in
    `BOARD`, tests that the target is empty, and does `PLACE`. Go adds `{ "name": "PASS", "do": [] }`.
  - A one-option promotion is a plain `SET_CODE`.
  - Atomic pawn captures carry no promotion steps.
  - Checkers men's jumps: `if` the end zone, then `SET_CODE KING` and `END_TURN`.

- [ ] **Step 1: Write the checkers tests first** (they pin Frank's answer 5) in
  `tests/checkers/mod.rs`:
  - `a_crowning_jump_is_forced_like_any_capture`;
  - `a_multi_jump_may_end_with_a_crowning_jump`;
  - `crowning_ends_the_turn_even_when_the_new_king_could_jump`.

  Run: `cargo test -p modern-chess-engine checkers`. Expected: these FAIL on today's spec, and
  the rest PASS.
- [ ] **Step 2: Rewrite the specs one family at a time.** After each family, run
  `cargo test -p modern-chess-engine` and keep it green. `catalogue_counts` must stay
  identical. Re-run `node apps/docs/scripts/derive-boards.mjs` (its header says how) and
  confirm the book's prepared boards still build.
- [ ] **Step 3: Run every Rust test, the lint, and the speed run, as in Task 3 Step 4.**
  Expected: all PASS; counts identical; times within 5% of `main`.
- [ ] **Step 4: Commit:** `git commit -m "feat(engine): every catalogue game written as actions and steps"`.
  Commit once per family if that is clearer.

---

### Task 5: Rewrite every other spec, and delete the old forms

**Files:**
- Modify:
  - the specs inside tests: `crates/engine/src/tests/**/*.json`, and the JSON strings in
    `crates/engine/src/tests/**/mod.rs`;
  - `crates/session/tests/fixtures/*.json`, `crates/game/tests/fixtures/*.json`, and the spec
    strings in `crates/session/tests/map_view.rs`;
  - `apps/docs/src/boards/catan_sampler.json`, `apps/docs/examples/*.json`, and the
    `boards.derived.json` recipes if needed;
  - `apps/web/src` test fixtures that embed a spec, if any.
- Delete:
  - `crates/engine/src/logic/game/ways/legacy.rs`;
  - `crates/engine/src/logic/blueprint/side_effects/`;
  - the old move parts in `move_blueprint/{mod.rs,build.rs,generate.rs}`: actions keyed by
    state, modifiers, side effects;
  - `shared/effect.rs`'s `Effect` and `EffectMetadata`, keeping `BoardChange`;
  - `specs/game/piece/{action.rs,modifier.rs,side_effect.rs}`;
  - the constants `MOVE`, `CAPTURE`, `TRANSFORM`, `DROP`, `PASS`, `CONVERT`, `COPY_SOURCE`,
    `EXPLODE`, `SET_STATE` where now unused;
  - `Rules.allow_pass`, `Rules.unlimited_supply`, the per-piece drop conditions and
    `compute_drop_squares`.
- Modify:
  - `crates/engine/src/specs/game/old_forms.rs`: the refusals;
  - `game.rs`: `actions` is required and must not be empty;
  - `move.rs`: `do` is required;
  - the session shim: `drops`/`can_pass` from ways only.

**Interfaces:**
- **Produces:** `OldForm` refusals, each naming the replacement:
  - a move's `actions`, `conditions`, `modifiers`, `side_effects`: replaced by `do`;
  - `allow_pass`: replaced by an action with an empty `do`;
  - `unlimited_supply`: replaced by an action that does `PLACE`;
  - `drop_conditions`: replaced by tests in the drop action.

- [ ] **Step 1: Write the failing refusal test**
  `old_turn_forms_are_refused_naming_their_replacement` in `tests/action_blocks/mod.rs`, one
  case per old field above. Run it, and expect FAIL: the fields are still accepted.
- [ ] **Step 2: Rewrite every remaining spec.** Keep what each test checks. A test that existed
  to check an old form, such as `side_effects` or `transformation`, now checks the same
  behaviour written in steps.
- [ ] **Step 3: Delete the old forms, the adapter and the dead code, and add the refusals.**
  `rg -n '"CAPTURE"|"TRANSFORM"|EffectMetadata|unlimited_supply|allow_pass' crates apps/cli`
  must find no code, only refusals and docs.
- [ ] **Step 4: Run every Rust test and the lint.**
  Run: `cargo test --workspace && make lint-panics`
  Expected: all PASS.
- [ ] **Step 5: Commit:**
  `git commit -m "refactor(engine)!: delete the old turn forms; they are refused with their replacement"`

---

### Task 6: Session and wasm speak choose and cancel

**Files:**
- Modify:
  - `crates/session/src/{session.rs,view.rs,lib.rs}`;
  - every `crates/session/tests/*.rs`, including `support/mod.rs` (the robustness driver);
  - `crates/wasm/src/lib.rs` and `crates/wasm/tests/identical.rs`.

**Interfaces:**
- **Consumes:** Task 3's `Answer`, `Choice` and `Game::choice`/`shown_pieces`.
- **Produces:**
  - `Session::choose(&mut self, answer: Answer) -> Result<(), String>` and
    `Session::cancel(&mut self) -> Result<(), String>`.
  - `select`, `make_move`, `drop_piece`, `pass` and `promote` are removed.
  - Click feel lives in `choose(Answer::Place(p))`:
    - If `p` answers the open choice, choose it.
    - Otherwise, if cancelling is allowed, cancel; then if `p` answers the fresh choice, choose
      it, and if not, the call succeeds having only cancelled.
    - Otherwise it fails with `"that is not a choice right now"`.
  - `View.choice: Choice` replaces `selected`, `destinations`, `promotion`, `drops`,
    `can_pass` and `supply`. `passes`, `hand`, `hands` and the rest stay. `pieces` come from
    `shown_pieces`.
  - **wasm `Board`:**
    - `choose(answer: &str)` takes an `Answer`'s JSON, `{"place":"4,2"}`; malformed JSON is a
      `JsError`.
    - `cancel()`; the other turn methods are removed.

- [ ] **Step 1: Write the failing tests** in `crates/session/tests/play.rs`:
  - `clicking_another_piece_while_a_move_is_open_chooses_it`;
  - `clicking_a_place_that_answers_nothing_cancels`;
  - `a_drop_is_two_answers_hand_then_place`;
  - `pass_is_an_action_answer`;
  - `promotion_is_a_code_answer`;
  - `resume_keeps_an_open_promotion`.

  In `view.rs`, add `the_view_shows_one_choice_object`, with the exact JSON field names from
  the design. Run `cargo test -p modern-chess-session`. Expected: FAIL.
- [ ] **Step 2: Implement the session API and view, and the wasm methods.** Rewrite every
  session test, and the robustness driver, onto `choose`/`cancel`. The driver picks random
  answers from `view.choice` and also fires garbage answers.
- [ ] **Step 3: Run the tests.**
  Run: `cargo test -p modern-chess-session -p modern-chess-wasm && make test-damage`
  Expected: all PASS.
- [ ] **Step 4: Commit:**
  `git commit -m "feat(session)!: one choose and cancel protocol, and one choice in the view"`

---

### Task 7: The game service's commands

**Files:**
- Create: `crates/game/migrations/0005_actions.{up,down}.sql`.
- Modify:
  - `crates/game/src/domain/command.rs`;
  - `crates/game/src/http/play.rs`;
  - `crates/game/src/store/{command.rs,locked.rs}` if needed;
  - `crates/game/tests/{play.rs,page_and_commands.rs,store.rs}` and the fixtures.

**Interfaces:**
- **Consumes:** Task 6's `Session::choose`/`cancel` and `View.choice`.
- **Produces:**
  - `CommandKind`, serde-tagged `"type"`:
    - `Choose { place: Option<String>, code: Option<String>, hand: Option<String>, action: Option<String> }`;
    - `Cancel`;
    - `Resign`.
  - The old variants are removed.
  - The API refuses with 400, before logging:
    - a `choose` with not exactly one answer key;
    - any of its strings over `MAX_POSITION_BYTES` (64).
  - Logged legacy rows may carry `hand` and `place` together. They are read, never accepted
    from the API.
  - `snapshot()` replaces `view.choice` with an empty `Choice` for anyone but the player to
    move. This fixes today's leak of `promotion`.
  - **Migration `0005`:**
    - logged `select`/`move` become `{"type":"choose","place":…}`;
    - `promote` becomes `{"type":"choose","code":…}`;
    - `pass` becomes `{"type":"choose","action":"PASS"}`;
    - `drop` becomes `{"type":"choose","hand":…,"place":…}`;
    - the `.down.sql` reverses each of these;
    - stored match states are not migrated (Frank's answer 4).

- [ ] **Step 1: Write the failing tests** in `tests/play.rs`:
  - `a_choose_command_needs_exactly_one_answer` (none, or two: 400);
  - `every_answer_string_is_limited_to_64_bytes` (a 65-byte code, hand or action: 400);
  - `the_choice_is_hidden_from_the_player_not_to_move` (including during a promotion);
  - `a_drop_is_two_choose_commands`;
  - `a_promotion_is_a_choose_code_command`;
  - `cancel_is_logged_and_clears_the_choice`.

  In `tests/store.rs`, add `the_migration_turns_logged_commands_into_choose`. Run
  `cargo test -p modern-chess-game`, which needs `make db-up`. Expected: FAIL.
- [ ] **Step 2: Implement the command type, the validation, the snapshot filter and the
  migration.** Rewrite the HTTP test helpers (`select()`, `mv()`, the drop and pass builders)
  onto `choose`.
- [ ] **Step 3: Run the tests.**
  Run: `cargo test -p modern-chess-game`
  Expected: all PASS.
- [ ] **Step 4: Commit:**
  `git commit -m "feat(game)!: choose and cancel commands, with the choice hidden from the waiting player"`

---

### Task 8: Web app and board element

**Files:**
- Modify:
  - `apps/web/src/api/types.ts`;
  - `apps/web/src/pages/play.tsx`;
  - `apps/web/src/components/{remote-board.tsx,play-island.tsx,board-preview.tsx}`;
  - `apps/web/src/board.d.ts` and `apps/web/src/test/fixtures.ts`;
  - the web tests: `pages/play.test.tsx`, `components/play-island.test.tsx`,
    `components/board-remote.test.ts`, `components/board-local.test.ts`;
  - `apps/docs/theme/modern-chess-board.js` (+ `.css`);
  - `apps/docs/theme/test/smoke.mjs`.

**Interfaces:**
- **Consumes:** Task 6's view `choice` and wasm `choose`/`cancel`, and Task 7's command shapes.
- **Produces:**
  - `Command` is `{type:"choose", place|code|hand|action}`, `{type:"cancel"}` or
    `{type:"resign"}`.
  - `BoardView.choice: { chosen, start, places: {name, for}[], codes, hand, actions: {name, plays}[] }`,
    replacing `selected`, `destinations`, `promotion`, `drops`, `can_pass` and `supply`.
  - **The board element emits `choose` (detail: one answer) and `cancel`.**
    - A click on any node while playable emits `choose {place}`; the session decides what it
      means.
    - A hand chip emits `choose {hand}`.
    - In local mode, `act(["choose", JSON.stringify(answer)])`.
    - `select`, `move`, `dropPiece`, `promote` and the `dropCode` prop are removed.
    - The local panel shows `codes` (the old promotion panel) and buttons for `actions`. Pass
      is one of these.
    - Marks: `MOVE`/`PLACE` places get `.mcb-dest`, `PIECE` places stay unmarked, and `start`
      gets `.mcb-selected`. Keep today's look on Go, where `PLACE` places are not dotted if
      they are not dotted today. Check this in the browser in Task 10.
  - **`PlayIsland`:**
    - promotion buttons come from `choice.codes` and send `choose {code}`;
    - one button per `choice.actions` entry sends `choose {action}` (Pass among them);
    - hand chips send `choose {hand}`;
    - hints read `choice` in place of the old fields.

- [ ] **Step 1: Respell the web tests and the board element tests onto the new shapes first.**
  Keep what each checks, including `with_two_supply_pieces_one_has_to_be_chosen_first`, which
  now answers an action. Run `pnpm --filter web test`. Expected: FAIL.
- [ ] **Step 2: Implement the types, the page, the island, the remote and local board, and
  `smoke.mjs`.**
- [ ] **Step 3: Run the tests.**
  Run: `pnpm --filter web test && pnpm --filter web typecheck && bash apps/docs/build.sh && node apps/docs/theme/test/smoke.mjs`
  Expected: all PASS. Use the package's actual script names if they differ, and say so in the
  report.
- [ ] **Step 4: Commit:**
  `git commit -m "feat(web)!: the board and the play page answer one choice"`

---

### Task 9: The spec reference and the book teach actions and steps

**Files:**
- Modify:
  - `specs/SPEC_FORMAT.md`;
  - `apps/docs/src/{01-first-game,02-pieces-that-move,03-promotion-and-zones,04-conditions-and-state,06-zones-and-blocking,07-the-hand,08-side-effects,09-forced-captures,10-placing-stones,11-design-your-own,12-maps,reference,known-limitations}.md`;
  - `apps/docs/README.md`: the test entry points, and drop the stale `crates/wasm/tests/boards.rs`
    mention.

- [ ] **Step 1: Update `SPEC_FORMAT.md`.** Document actions, steps, choices, effects, `BOARD`,
  `EACH`, `END_TURN`, the capture and pass definitions, and has-moved. Add each refusal and
  `OldForm` to the refusal tables. Remove the deleted forms.
- [ ] **Step 2: Rewrite the book's JSON examples and prose.**
  - Chapter 8 becomes effects.
  - Chapter 9 explains `END_TURN` and the standard checkers rules.
  - Chapter 10 replaces the supply with a placing action.
  - Reword every "select" passage listed in the design's protocol section.
- [ ] **Step 3: Build the book.**
  Run: `bash apps/docs/build.sh && node apps/docs/theme/test/smoke.mjs`
  Expected: it builds, and the smoke test passes.
- [ ] **Step 4: Commit:** `git commit -m "docs: the spec reference and the book teach actions, steps and effects"`

---

### Task 10: Prove it (controller)

- [ ] **Run the whole suite.** Run `cargo test --workspace` (with `make db-up`),
  `make lint-panics`, `make test-damage`, the web tests and typecheck, and the book build plus
  smoke test. Expected: all green.
- [ ] **Speed, side by side with `main`.** Three runs each, interleaved. Expected: medians
  within 5% on chess, Shogi and Go, with identical counts.
- [ ] **In the book, in a browser.** Each of these plays:
  - chess: castling, en passant, promotion;
  - Shogi: a drop, an optional promotion;
  - Go: a stone, a pass;
  - checkers: a double jump, and a crowning jump that ends the turn;
  - Ataxx and Atomic.
- [ ] **One live match through the web app**, including a promotion seen from both seats.
- [ ] **Final whole-branch review**, then the PR, with Frank's go-ahead to push.
