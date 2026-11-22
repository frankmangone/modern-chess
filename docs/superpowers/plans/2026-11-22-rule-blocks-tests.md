# Rule blocks, slice 1: tests — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every condition in a spec is written as a tree of elementary test blocks, and every old condition form is gone from the spec format and the engine.

**Architecture:** New spec types (`TestSpec`, `RegionSpec`, `PlaceSpec`, `FilterSpec`) are compiled once per spec into a `Test` tree with each offset already turned per player, and one `judge` serves moves, actions, modifiers, drops and repeats. The migration runs in three moves so every task ends green: the new blocks are accepted beside the old forms (Tasks 1–2), every spec is rewritten (Tasks 3–4), and the old forms are deleted with refusals for them (Task 4).

**Tech Stack:** Rust (serde, the `modern-chess-engine` crate), JSON specs, the mdBook docs book (`apps/docs`).

**Spec:** `docs/superpowers/specs/2026-11-22-rule-blocks-design.md` (section "Slice 1: tests"; the route section is context only).

## Global Constraints

- JSON shapes exactly as in the design's tables: tests `{ "test": "ALL"|"ANY"|"NOT"|"COUNT"|"ATTACKED"|"IN_ZONE", ... }` with fields `of`, `in`, `where`, `min`, `max`, `by`, `at`, `zone`; regions `{ "region": "AT"|"PATH"|"AROUND"|"LINE", ... }` with fields `at`, `from`, `to`, `ends`, `of`, `through`, `along`; places `{ "place": "START"|"TARGET", "offset": [dx, dy] }`; filters `{ "owner", "codes", "flag", "moved" }`.
- Defaults: `COUNT.min` = 1, `COUNT.max` = no limit, `COUNT.where` = every piece; `ALL` of an empty list holds, `ANY` of an empty list does not; `ATTACKED.by` = `"ENEMY"`; `owner` = `"ANY"`; `PATH.ends` = none; `AROUND.through` = `"adjacent"`; a repeat's `until` = `COUNT` in `AT(TARGET)` (the next square is occupied).
- `owner` takes `ALLY`, `ENEMY`, `ANY`; `by` takes `ENEMY` only; `ends` entries are `FROM`, `TO`.
- `START` is the moving piece's square; `TARGET` is where the move lands (for a slide, the square being judged; for a repeat's `until`, the next square; for a drop, the drop square). `START` in a drop condition is refused.
- Offsets and `LINE.along` are written from the neutral player's view and turned per player like a step.
- A region that cannot be formed (a place off the board; a `PATH` that is not a straight line, or crosses a hole or the edge) makes `COUNT` and `ATTACKED` over it **false**; `NOT` flips that as usual.
- A straight line steps by -1, 0 or +1 on each axis (today's `Board::path` walk).
- Offsets, `PATH` and `LINE` are grids only: refused on a map.
- Zones: `"zones": [{ "name": "PALACE", "places": { "RED": [...], "BLACK": [...] } }]`, places written as today (coordinates on a grid, names anywhere).
- Nothing unknown is read as false: an unknown test/region/place kind, an unknown or misspelled field in a block, an unknown zone, a code in `codes` no piece has, `min` above `max`, an unknown `AROUND` label are refused when the spec is read, with a `GameSpecError` in the style of today's.
- Grid speed: `cargo run --release -p modern-chess-engine --example speed` stays within 5% of `main` on chess, Shogi and Go (compared side by side, median of three runs each).
- Every existing test keeps what it checks; tests are respelled, not weakened.
- `make lint-panics` stays clean. Conventional commits ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. pnpm for JS. Never push.

## Review Focus

1. A slide whose test names `TARGET` (Janggi's cannon: "the landing square is not a cannon") is judged afresh on every square of the slide, not once — Task 1 test `count_at_target_is_judged_on_every_square_of_a_slide`.
2. Offsets and `along` for a player turned 90° (4-player chess) land on the right square — Task 1 test `an_offset_is_turned_for_a_sideways_player`, Task 2 test `line_along_is_turned_for_each_player`.
3. `NOT` over a region that cannot be formed (a horse's leg square off the board) holds, while `COUNT` over it fails — Task 1 tests `not_over_an_off_board_place_holds` and `count_over_a_broken_path_fails`.
4. A misspelled field inside a block (`"wher"`, `"offest"`) is refused, not ignored — Task 1 test `a_misspelled_field_in_a_block_is_refused`.
5. After the rename to `zones`, the session view still lists every zone with its players' places — Task 2 test in `crates/session/tests/view.rs`: `the_view_lists_zones_written_as_zones`.

---

### Task 1: Test blocks in move and action conditions

**Files:**
- Create: `crates/engine/src/specs/game/test_block.rs` (spec types), `crates/engine/src/logic/blueprint/test_block/mod.rs` (compiled `Test`, `Region`, `Place`, `Filter`), `.../test_block/build.rs` (spec → compiled, per player), `.../test_block/judge.rs` (`judge`), `crates/engine/src/tests/test_blocks/mod.rs`, `crates/engine/src/tests/test_blocks/spec.json`
- Modify: `crates/engine/src/specs/game/piece/move.rs`, `.../piece/action.rs` (condition lists accept a test or a legacy condition), `crates/engine/src/logic/blueprint/move_blueprint/{mod.rs,build.rs,generate.rs}`, `crates/engine/src/specs/game/game.rs` (validation), `crates/engine/src/tests/mod.rs`

**Interfaces:**
- Produces (spec): `TestSpec` (serde internally tagged on `"test"`, variants `All { of: Vec<TestSpec> }`, `Any { of }`, `Not { of: Box<TestSpec> }`, `Count { r#in: RegionSpec, r#where: Option<FilterSpec>, min: Option<u16>, max: Option<u16> }`, `Attacked { r#in, by: Option<String> }`, `InZone { at: PlaceSpec, zone: String }`); `RegionSpec` tagged on `"region"` (`At { at }`, `Path { from, to, ends: Vec<String> }`, `Around { of, through: Option<String> }`, `Line { through, along: ExtendedPosition }`); `PlaceSpec { place: String, offset: Option<ExtendedPosition> }`; `FilterSpec { owner: Option<String>, codes: Vec<String>, flag: Option<String>, moved: Option<bool> }`. Every block type refuses unknown fields.
- Produces (spec, transitional): `ConditionEntry` (untagged: `Test(TestSpec)` tried first, then `Legacy(ConditionSpec)`); `MoveSpec.conditions` and `ActionSpec.conditions` become `Vec<ConditionEntry>`.
- Produces (engine): `pub enum Test` (compiled), `pub struct TestContext<'a> { pub game: &'a Game, pub player: &'a str, pub start: Option<&'a Position>, pub target: &'a Position }`, `pub fn judge(test: &Test, ctx: &TestContext<'_>) -> bool`, and `Test::build(spec: &TestSpec, players: &[PlayerSpec], step: Option<&StepSpec>, labels: &mut Labels) -> Test` (offsets and `along` turned per player; a `PATH`'s line direction is the unit of (`to` − `from`) where `TARGET` counts as `START` + step, computed per player at build time).
- Produces: `MoveBlueprint.conditions` and `ActionBlueprint.conditions` hold `Vec<Rule>` where `enum Rule { Legacy(Condition), Test(Test) }`, judged in order.

- [ ] **Step 1: Write the failing tests** in `tests/test_blocks/mod.rs`, against a small 8×8 grid spec in `spec.json` (two players WHITE identity / BLACK 180°, plus a third player SIDE turned 90° for the turning test, pieces whose moves carry the tests below; add pieces as needed). One test per behaviour:
  - `count_at_an_offset_sees_an_enemy` (`COUNT in AT(START+[1,1]) where owner ENEMY` allows the move only with an enemy there)
  - `count_with_codes_flag_and_moved_filters` (each filter field narrows the count; `moved: false` matches a piece that never moved)
  - `count_bounds_min_and_max` (`min: 0, max: 0` holds on an empty path; `min: 1, max: 1` holds with exactly one piece)
  - `count_at_target_is_judged_on_every_square_of_a_slide` (a sliding move with `NOT(COUNT in AT(TARGET) where codes [X])` stops offering only the squares holding an X)
  - `path_counts_between_start_and_target_without_ends` and `path_ends_include_from_and_to`
  - `count_over_a_broken_path_fails` (a `PATH` crossing a disabled square, and a `PATH` along `[2, 1]`: `COUNT … min 0 max 0` is false)
  - `not_over_an_off_board_place_holds` (`NOT(COUNT in AT(START+[0,1]))` holds at the board's edge)
  - `around_counts_adjacent_and_orthogonal_neighbours`
  - `attacked_sees_an_enemy_attack_on_the_region`
  - `in_zone_reads_the_movers_part_of_a_zone`
  - `all_any_not_and_their_empty_cases` (`ALL []` holds, `ANY []` fails, `NOT` flips)
  - `an_offset_is_turned_for_a_sideways_player`
  - `a_misspelled_field_in_a_block_is_refused` (`"wher"` in a `COUNT`, `"offest"` in a place: `GameSpec` parse fails)
  - `refusals_when_the_spec_is_read`: unknown zone; a code in `codes` no piece has; `min` above `max`; an `AROUND` label the board lacks; `owner: "FRIEND"`; `by: "ALLY"`; `ends: ["MIDDLE"]`; `place: "HERE"` — each is a `GameSpecError` whose text names the offending value
  - `grid_only_blocks_are_refused_on_a_map` (an offset, a `PATH`, a `LINE` in a map spec)

- [ ] **Step 2: Run them to see them fail**
  Run: `cargo test -p modern-chess-engine test_blocks`
  Expected: compile failure (no `test_blocks` module / unknown `"test"` field) — then, once the module compiles against stubs, every test FAILS.

- [ ] **Step 3: Add the spec types, `ConditionEntry`, the compiled tree, `Test::build` and `judge`**, and route move-level and action-level conditions through `Rule`. Validation goes in `GameSpec::validate` (a new `validate_tests` that walks every test in the spec) using the existing `GameSpecError` variants where they fit (`UnknownPieceInRule`, `NotOnAMap`, `UnknownLabel`, `ValueOutOfRange`, …) and one new variant `InvalidTest(String)` for the rest. Judging must not allocate for `AT` regions (the common case). `ATTACKED` uses `Game::attacked_by` for each opponent of the mover, as `not_attacked.rs` does. `IN_ZONE` uses `Game::check_position_condition_for_player_str` with the mover.

- [ ] **Step 4: Run the new tests and the whole engine suite**
  Run: `cargo test -p modern-chess-engine`
  Expected: all PASS (old specs still use legacy conditions and are unaffected).

- [ ] **Step 5: Commit**
  `git commit -m "feat(engine): tests as blocks in move and action conditions"`

### Task 2: Test blocks everywhere else, and zones

**Files:**
- Modify: `crates/engine/src/specs/game/piece/{modifier.rs,move.rs,piece.rs}`, `crates/engine/src/specs/game/{game.rs,condition.rs}`, `crates/engine/src/logic/blueprint/{piece_blueprint.rs,move_blueprint/*}`, `crates/engine/src/logic/game/{legal.rs,cell_conditions.rs}`, `crates/session/src/session.rs`, `crates/session/tests/view.rs`, `crates/engine/src/tests/test_blocks/*`

**Interfaces:**
- Consumes: Task 1's `TestSpec`, `ConditionEntry`, `Test`, `Test::build`, `TestContext`, `judge`, `Rule`.
- Produces: `ModifierSpec.conditions: Vec<ConditionEntry>`; `PieceSpec.drop_conditions: Vec<TestSpec>` (all must hold; judged with `start: None`, `target` = drop square); `RepeatSpec.until: Option<UntilSpec>` where `UntilSpec` is untagged `Test(TestSpec)` | `Legacy(String)`; `GameSpec.zones: Vec<ZoneSpec>` with `ZoneSpec { name: String, places: HashMap<String, Vec<PositionSpec>> }`, merged at build time into the same zone table `IN_ZONE`, win conditions and the session view read (today `rules.conditions`). The legacy forms (`conditions` POSITION list, `drop_restrictions`, `restrictions`, string `until`, named modifier conditions) keep working until Task 4.

- [ ] **Step 1: Write the failing tests**
  - `tests/test_blocks`: `modifier_conditions_take_any_test` (a promotion gated by `COUNT in AT(START+[0,-1]) where owner ALLY` fires only then); `drop_conditions_must_all_hold` (a drop with `NOT(COUNT in LINE(TARGET, [0,1]) where owner ALLY codes [PAWN])` is refused on a file holding an ally pawn and allowed elsewhere); `start_in_a_drop_condition_is_refused`; `repeat_until_takes_a_test` (a slide with `until: ANY []` runs to the edge over empty squares and stops before a piece as the move's actions decide; a slide with `until: COUNT in AT(TARGET) where owner ENEMY` stops after reaching an enemy); `line_along_is_turned_for_each_player`; `zones_are_read_from_zones` (`IN_ZONE` and a `PIECE_IN_ZONE` win condition both see a zone written under `zones`); `a_zone_named_twice_is_refused`.
  - `crates/session/tests/view.rs`: `the_view_lists_zones_written_as_zones`.

- [ ] **Step 2: Run them to see them fail**
  Run: `cargo test -p modern-chess-engine test_blocks && cargo test -p modern-chess-session --test view`
  Expected: the new tests FAIL (unknown fields / zones missing from the view).

- [ ] **Step 3: Implement** the four new places for tests and the `zones` list. A zone name declared in both `zones` and the legacy `conditions` is refused as a duplicate. Grid zone names must name enabled squares, as POSITION names do today (`validate_map_positions`).

- [ ] **Step 4: Run the engine and session suites**
  Run: `cargo test -p modern-chess-engine && cargo test -p modern-chess-session`
  Expected: all PASS.

- [ ] **Step 5: Commit**
  `git commit -m "feat(engine): tests in modifiers, drops and repeats, and zones"`

### Task 3: Rewrite the catalogue specs in blocks

**Files:**
- Modify: every `crates/engine/specs/*.json` that writes a condition, a zone, `restrictions`, `drop_restrictions` or an `until` (21 files)
- Create: `crates/engine/src/tests/catalogue_counts/mod.rs`

**Interfaces:**
- Consumes: every block from Tasks 1–2.
- Produces: catalogue specs that use only `zones`, test blocks, `drop_conditions` and test `until`s. The legacy reader still exists (Task 4 removes it).

- [ ] **Step 1: Pin today's behaviour before touching a spec.** Write `catalogue_counts::every_game_keeps_its_move_counts`: for every `crates/engine/specs/*.json`, count the legal moves summed over every ply to depth 2 from the start (depth 3 for chess, crazyhouse, xiangqi, janggi_cannon, shogi, solidarity), using the same walk as `examples/speed.rs`, and assert against a table of numbers recorded by running it on the **unchanged** specs. Also confirm that each row of the design's "As blocks" table has a behaviour test today (horse leg in `tests/xiangqi`, cannon screen in `tests/janggi*`/`tests/path_count`, castling with an absent rook in `tests/castling*`, pawn double step in `tests/chess`-family tests, nifu in `tests/shogi`, palace in `tests/xiangqi`); add the missing ones, passing on the unchanged specs.
- [ ] **Step 2: Run it on the unchanged specs**
  Run: `cargo test -p modern-chess-engine catalogue_counts`
  Expected: PASS (the table was recorded from these specs).
- [ ] **Step 3: Rewrite each spec** using the design's "As blocks" table: zone conditions become `zones` entries and `IN_ZONE` tests; `DEPENDS_ON 0` on a pawn's double step becomes `NOT(COUNT in AT(START+[0,1]))` (Solidarity's double step also keeps its other condition); `"until": "NEVER"` becomes `"until": { "test": "ANY", "of": [] }`; castling's path counts need `"min": 0`; piece `restrictions` become an `IN_ZONE(TARGET, …)` test on each of that piece's moves (not on `threat_only` moves); `drop_restrictions` entries become the negated `drop_conditions`.
- [ ] **Step 4: Run the engine suite**
  Run: `cargo test -p modern-chess-engine`
  Expected: all PASS, `catalogue_counts` included, with no edit to its table.
- [ ] **Step 5: Commit**
  `git commit -m "refactor(games): write every catalogue rule in test blocks"`

### Task 4: Rewrite the remaining specs, and delete the old forms

**Files:**
- Modify: inline and file specs in `crates/engine/src/tests/**` (including `tests/conditions/spec.json`), `crates/session/tests/**`, `crates/game/tests/**`; book boards `apps/docs/src/boards/*.json`; `crates/engine/src/specs/game/{piece/*.rs,condition.rs,game.rs}`; `crates/engine/src/logic/blueprint/**`; `crates/engine/src/logic/game/{legal.rs,cell_conditions.rs}`; `crates/engine/src/shared/constants.rs`
- Delete: `crates/engine/src/logic/blueprint/conditions/` (every evaluator), `ConditionKind`, `ConditionEntry`/`UntilSpec` legacy variants, `DropRestriction*`, zone restrictions, the `STATE` condition type, the top-level `conditions` field

**Interfaces:**
- Consumes: Tasks 1–3.
- Produces: `MoveSpec.conditions`, `ActionSpec.conditions`, `ModifierSpec.conditions`: `Vec<TestSpec>`; `RepeatSpec.until: Option<TestSpec>`; `Rule` collapses to `Test`. The spec format has no `condition` objects, no `conditions` top-level list, no `drop_restrictions`, no `restrictions`.

- [ ] **Step 1: Write the failing refusal tests** in `tests/test_blocks`: `old_forms_are_refused` — a spec with a top-level `conditions` list, a piece with `drop_restrictions`, a piece with `restrictions`, a move condition written `{ "condition": "PIECE_AT", ... }`, and an `until` written as a string are each refused, and each error names the replacement (`zones`, `drop_conditions`, an `IN_ZONE` test on each move, a test block, a test). Replace the existing "unknown condition blocks the move" test(s) with refusals.
- [ ] **Step 2: Run them to see them fail**
  Run: `cargo test -p modern-chess-engine test_blocks::old_forms_are_refused`
  Expected: FAIL (old forms still accepted).
- [ ] **Step 3: Rewrite every remaining spec** (tests, session, game service, book boards) the way Task 3 did, then **delete the legacy code paths** listed above. `ConditionEvalContext`'s `valid_move_ids` and `DEPENDS_ON` go with them.
- [ ] **Step 4: Run every suite that reads specs**
  Run: `cargo test --workspace && make lint-panics && make test-damage && (cd apps/docs && ./build.sh) && node apps/docs/theme/test/smoke.mjs`
  Expected: all PASS (`make test-damage` about 3 minutes; the game service's DB tests need `make db-up`). `catalogue_counts` still passes with an unchanged table.
- [ ] **Step 5: Commit**
  `git commit -m "refactor(engine): drop the old condition forms"`

### Task 5: The spec reference and the book teach the blocks

**Files:**
- Modify: `specs/SPEC_FORMAT.md`; `apps/docs/src/{02-pieces-that-move,03-promotion-and-zones,04-conditions-and-state,05-winning-and-drawing,06-zones-and-blocking,07-the-hand,09-forced-captures,10-placing-stones,11-design-your-own,12-maps,known-limitations,reference}.md` wherever they show conditions, zones, restrictions or `until`

- [ ] **Step 1:** In `SPEC_FORMAT.md`, replace the sections `conditions — global named conditions`, `Move-level conditions` / `Available conditions`, `Zone restrictions`, `Drop restrictions`, the `until` row of `repeat`, the modifier `conditions` row, the "Removed primitives" table and the known limitations they made true, with: a `zones` section; a "Tests" section with the design's three tables (tests, regions, places/filters), the "cannot be formed" rule, the defaults, the refusals; where tests appear; and the "As blocks" table as the migration guide. Every JSON example in the file uses the blocks.
- [ ] **Step 2:** Rewrite the book chapters' prose and JSON the same way. Chapter 04 teaches `COUNT`/`NOT`/regions/places/filters with the en passant and castling examples; chapter 06 teaches zones and `IN_ZONE`; chapter 07 teaches `drop_conditions` with nifu.
- [ ] **Step 3: Check the book**
  Run: `(cd apps/docs && ./build.sh) && node apps/docs/theme/test/smoke.mjs && grep -rnE '"condition":|drop_restrictions|"restrictions"|PIECE_AT|LEG_EMPTY' apps/docs/src specs/SPEC_FORMAT.md`
  Expected: build and smoke PASS; the grep prints nothing.
- [ ] **Step 4: Commit**
  `git commit -m "docs: teach rules as test blocks"`

### Task 6: Prove it (controller)

- [ ] **Step 1: Speed.** Build `examples/speed.rs` on `main` and on this branch; run them interleaved, three runs each; record medians. Expected: each game within 5% of `main`, identical move counts.
- [ ] **Step 2: Browser.** In the book's "Try it" (`apps/docs`, `./build.sh`, serve `book/`), play chess castling and en passant, Xiangqi's horse and palace, Janggi's cannon, a Shogi drop refused by nifu, and a solidarity move. Expected: each behaves as on `main`.
