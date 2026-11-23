# Unknown fields refused everywhere Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every spec type refuses a key it does not know, and the old-form machinery (placeholders, `OldForm`, `END_TURN`) is deleted, so a retired form is just an unknown key with serde's own message.

**Architecture:** `#[serde(deny_unknown_fields)]` on the nine types that lack it (Task 1, with a table test and every non-compliant spec fixed); then delete `old_forms.rs` and everything that existed to refuse old text, converting the tests that matched `OldForm` (Task 2); then a guard test that the whole specs printed in the book read (Task 3).

**Tech Stack:** Rust (serde, serde_json), `cargo test`; docs are Markdown (`specs/SPEC_FORMAT.md`, `apps/docs/src`).

**Spec:** `docs/superpowers/specs/2026-11-26-unknown-fields-design.md`

## Global Constraints

- Errors are serde's own (`unknown field`, `unknown variant`, `invalid type`, `missing field`): no table of old keys, no hints (Frank's option 3).
- No change to what any valid spec means. A spec that only read because a key was ignored is fixed **in the spec**; never loosen the rule to excuse it.
- Every catalogue (`crates/engine/specs/*.json`), book (`apps/docs/examples`, `src/boards`), fixture and inline test spec must read under the strict reader.
- Commits are conventional (`fix:`, `refactor:`, `test:`, `docs:`) and end with `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.
- Branch is `feature/unknown-fields` (already holds the design). Do not push or open a PR without being asked.
- Suites: `cargo test -p modern-chess-engine -p modern-chess-session -p modern-chess-wasm` must pass after every task; `cargo test --workspace` before the end (the game and social crates need `TEST_DATABASE_URL`; the dev Postgres is on `localhost:5455`). Clippy warning count (`cargo clippy -p modern-chess-engine -p modern-chess-session --all-targets`) must not rise from 58.
- macOS: `sed -i ''`; there is no `timeout`.

## Review Focus

1. A misspelled *optional* key that has a default (`"star_at"` for `start_at`, `"capacty"` on a map place) used to vanish silently: it must be refused (Task 1 table).
2. A strict reader must still read what the engine writes: a spec serialized and read back (`GameSpec` is `Serialize`) must parse, since `skip_serializing` placeholders go away (Task 1 round-trip test).
3. A key typo inside a *raw* type serde reads through `try_from` (`RawBoardSpec`) or directly (`RawPlace`) is refused, not only typos on the public types (Task 1 table has both).
4. `"until": null` on a repeat still reads as no `until` once its custom reader is gone (Task 2).
5. A retired form's error names the offending key or value, so a person can find it (Task 2 table asserts the substring).

---

### Task 1: Refuse unknown keys on the nine types

**Files:**
- Modify: `crates/engine/src/specs/game/game.rs` (`GameSpec`), `board.rs` (`RawBoardSpec`), `map.rs` (`RawPlace`), `player.rs` (`PlayerSpec`, `PiecePositionSpec`), `piece/piece.rs` (`PieceSpec`), `piece/move.rs` (`MoveSpec`, `RepeatSpec`), `turns.rs` (`TurnSpec`)
- Create: `crates/engine/src/tests/unknown_fields/mod.rs`; add `mod unknown_fields;` to `crates/engine/src/tests/mod.rs` (alphabetical)
- Modify (as needed): any spec or inline test spec the strict reader refuses

**Interfaces:**
- Produces: test module `tests::unknown_fields` with `fn refused_naming(spec: &Value, key: &str)` (asserts `parse_game_spec_str` fails and the message contains `key`), which Task 2 extends.

- [ ] **Step 1: Write the failing tests** in `unknown_fields/mod.rs`.
  - `a_bogus_key_is_refused_at_every_level`: for each `(level, pointer)` below, load the spec, assert it reads untouched (positive control), set `"bogus": 1` on the object at the JSON pointer (`Value::pointer_mut`), and assert `refused_naming(.., "bogus")`. Chess (`./specs/chess.json`): `""` top level, `/board`, `/players/0`, `/players/0/starting_positions/0`, `/pieces/0`, `/pieces/0/moves/0`, `/turns`, and the rook's `/pieces/<index of code ROOK>/moves/0/repeat` (find the index by code). Kalah (`./src/tests/kalah/spec.json`): `/board/places/0` (a `RawPlace`).
  - `a_misspelled_optional_key_is_refused`: chess with `turns["star_at"] = 1` and Kalah with `board.places[0]["capacty"] = 3`; each refused naming the typo.
  - `every_catalogue_spec_survives_a_round_trip`: for each `./specs/*.json`, `parse_game_spec_str` → `serde_json::to_string` → `parse_game_spec_str` is `Ok`.
- [ ] **Step 2: Run** `cargo test -p modern-chess-engine unknown_fields` → the first two FAIL (specs accepted); the round trip passes.
- [ ] **Step 3: Add `#[serde(deny_unknown_fields)]`** to the nine types listed under Files. On `RawBoardSpec` it goes on the raw struct (`BoardSpec` reads through `try_from`); `deny_unknown_fields` cannot be combined with `flatten`, and none of the nine uses it.
- [ ] **Step 4: Run** the three tests → PASS. Then run the engine, session and wasm suites and the docs-board test (`cargo test -p modern-chess-session --test boards`; it needs `apps/docs/src/boards`, made by `node apps/docs/scripts/derive-boards.mjs` after the copies in `apps/docs/build.sh`). **Every spec that now fails is fixed in the spec** (the JSON file, or the `json!` in the test), by deleting the stray key or correcting its spelling; list each fix in the commit message. The game crate fixtures (`crates/game/tests/fixtures`) are covered by `cargo test -p modern-chess-game` with `TEST_DATABASE_URL`.
- [ ] **Step 5: Commit** `fix(engine): refuse unknown keys on every spec type`.

---

### Task 2: Retire the old-form machinery

**Files:**
- Delete: `crates/engine/src/specs/game/old_forms.rs`, `crates/engine/src/tests/old_end_forms/` (and its `mod old_end_forms;` in `tests/mod.rs`)
- Modify: `specs/game/mod.rs` (drop `pub mod old_forms;`), `game.rs`, `piece/piece.rs`, `piece/move.rs`, `parser.rs`, `action.rs`, `tests.rs` (in `specs/game`), `logic/rules/steps/build.rs`
- Modify tests: `tests/go/mod.rs` (`the_old_keys_are_refused_with_old_form`), `tests/test_blocks/mod.rs` (`old_forms_are_refused`), `tests/action_blocks/mod.rs` (the old-forms loop near line 676), `tests/every_move_rules/mod.rs` (the two `..._with_old_form` tests), `tests/bags/mod.rs` (`the_old_hand_forms_are_refused_naming_their_replacement`), `tests/stages/mod.rs` (`jump_enabled_...`, `the_end_turn_effect_...`), `tests/unknown_fields/mod.rs`
- Modify docs: `specs/SPEC_FORMAT.md` (lines ~928-930, ~1251, the "Old forms" section ~1914), `apps/docs/src/11-design-your-own.md` (error table, `OldForm(...)` row), `apps/docs/src/known-limitations.md` if it names old forms

**Interfaces:**
- Consumes: `refused_naming` from Task 1.
- Produces: no `OldField`, no `GameSpecError::OldForm`, no `EffectSpec::EndTurn`; `RepeatSpec.until: Option<TestSpec>` read by serde's default `Option` (so `null` is none).

- [ ] **Step 1: Write the failing test** `the_retired_forms_are_refused_with_serde_messages` in `unknown_fields/mod.rs`: a table of `(where, spec edit, substring the message must contain)`, each refused and checked with `refused_naming`. Each form is written on a copy of chess (any valid spec works, since the key is unknown whatever the game): top level `conditions`, `hand`, `hand_enabled`, `no_opponent_check`, `jump_enabled`, `mandatory_capture`, `allow_pass`, `leader`, `draw_conditions`, `stalemate_loses`, `win_conditions`, `forbid_repeated_positions`, `liberty_capture` → substring is the key; piece `demotes_to`, `drop_conditions`, `drop_restrictions`, `restrictions`, `unlimited_supply`; move `actions`, `conditions`, `modifiers`, `side_effects` → the key; rook repeat `until` as the string `"NOT_EMPTY"` → `NOT_EMPTY`; `until` as `{ "condition": "PIECE_AT" }` → `test`; effect `{ "effect": "END_TURN" }` pushed on a move's `do` → `END_TURN`. Add `a_repeat_until_of_null_reads_as_none`: rook repeat `until: null` reads, and `parsed.pieces[..].moves[0].repeat.as_ref().unwrap().until.is_none()`.
- [ ] **Step 2: Run** `cargo test -p modern-chess-engine unknown_fields` → the table FAILS (messages are the old-form texts naming replacements, not serde's).
- [ ] **Step 3: Delete the machinery.** Remove `old_forms.rs` and its module line; the 13 `old_*` fields of `GameSpec`, 5 of `PieceSpec`, 4 of `MoveSpec`, with their `use ... old_forms` lines; the `deserialize_with = "old_forms::until"` on `RepeatSpec.until` (keep `default` and `skip_serializing_if`); `GameSpecError::OldForm`; `old_form_refusal` in `parser.rs` (keep `map_refusal`); `EffectSpec::EndTurn`, its `validate` arm in `game.rs` and its arm in `logic/rules/steps/build.rs`; the `OldField` lines in `specs/game/tests.rs`'s `create_valid_game_spec`. Let the compiler list any other use.
- [ ] **Step 4: Convert the old-form tests** listed under Files: each keeps its inputs and asserts only that the form is refused (`refused_naming(&spec, &[the key or value])`), dropping `matches!(.., OldForm(_))` and the "names the replacement" assertions. Delete `old_end_forms` (its four keys are in Step 1's table). In `stages/mod.rs` keep `jump_enabled` and `END_TURN` refused naming themselves. Update `SPEC_FORMAT.md` and chapter 11 as the design says: no `OldForm` error is claimed anywhere; the "Old forms" section becomes "Retired forms" with the same "write instead" table and one sentence that a retired key is refused as an unknown field.
- [ ] **Step 5: Run** the engine, session and wasm suites, then `cargo test --workspace` with `TEST_DATABASE_URL` set → all pass; `cargo clippy -p modern-chess-engine -p modern-chess-session --all-targets` warnings ≤ 58; `cargo fmt --check` clean; `rg -n "OldForm|OldField|old_forms" crates apps specs` → no match.
- [ ] **Step 6: Commit** `refactor(engine): retire the old-form refusals; a retired key is an unknown field`.

---

### Task 3: The book's whole specs read

**Files:**
- Create: `crates/session/tests/book_specs.rs`
- Modify (as needed): `apps/docs/src/*.md`, `specs/SPEC_FORMAT.md`

**Interfaces:**
- Consumes: `modern_chess_session::Session::new(spec_json: &str) -> Result<Session, String>`.

- [ ] **Step 1: Write the test** `every_whole_spec_in_the_book_reads`: read every `apps/docs/src/*.md` and `specs/SPEC_FORMAT.md` (paths from `env!("CARGO_MANIFEST_DIR")`), pull out each fenced ```` ```json ```` block, parse it as JSON, and for each block that is an object holding both `"board"` and `"players"` assert `Session::new` is `Ok`, naming file and block number in the failure. Blocks the book prints on purpose as errors go in an explicit skip list in the test (file, block number, reason).
- [ ] **Step 2: Run** `cargo test -p modern-chess-session --test book_specs`. Any failure is a book example that a reader copying it would be refused on: fix the example in the chapter, never the rule.
- [ ] **Step 3: Commit** `test(session): the whole specs printed in the book read`.

> Task 3 is a plan-level addition, not in the design: it enforces "all specs comply" for the book's printed specs, which no test covers today. Drop it if unwanted; Tasks 1-2 stand alone.
