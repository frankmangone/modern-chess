# Link labels on maps, and the full Kalah Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A map's written links can carry a label of the spec's own choosing, and the full Kalah (with its capture) enters the catalogue.

**Architecture:** `MapSpec.links` is keyed by label name (`String`): the six built-in labels always, plus every label a written link names. `RawLink` gains a labelled one-way form and a labelled two-way form (`between`). Validation of steps, `AROUND` and `FOLLOW` asks the map for its labels. The board's link tables follow the map's labels. The Kalah capture is written with existing blocks, through two labels: `across` (the pit opposite) and `store` (the store a pit drains into).

**Tech Stack:** Rust (serde, serde_json), `cargo test`; docs are Markdown.

**Spec:** `docs/superpowers/specs/2026-11-26-link-labels-design.md`

## Global Constraints

- No change to what any existing spec means: a link without `label` is `next`, exactly as today; every catalogue, book and fixture spec must read as before (the whole engine, session and wasm suites).
- A labelled link joins two points. Refused when read, `InvalidMap`: an empty label, a label over 64 bytes (`MAX_NAME_BYTES`), a label that is one of `ends`, `paths`, `corners`, `areas`, `sides`. `next` stays writable and means the unlabelled form.
- A labelled link is never drawn as a line: the session's `lines` stays `lines` and `one_way` (the `next` links) only.
- Every feature is proved by a test seen to fail first. Commits are conventional and end with `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. Branch is `feature/link-labels` (holds the design and the Flying Knight start fix). Do not push or open a PR without being asked.
- Suites: `cargo test -p modern-chess-engine -p modern-chess-session -p modern-chess-wasm` after every task; `cargo test --workspace` needs `TEST_DATABASE_URL` (ask Frank to run it). Clippy warnings (`cargo clippy -p modern-chess-engine -p modern-chess-session --all-targets`) must stay at 59; `cargo fmt --check` clean.
- macOS: `sed -i ''`; there is no `timeout`.

## Review Focus

1. A written label that is a typo of a built-in one (`"nxt"`) is a new label, not an error; but a step, `AROUND` or `FOLLOW` naming a label no map has is refused with `UnknownLabel` (Task 1).
2. A labelled link round-trips: a spec read, written back and read again is the same map (Task 1).
3. A labelled link never reaches the drawing: Kalah's `across` links add no lines to the session view (Task 2).
4. The capture: last stone in an empty own pit with stones across takes both; no capture when the pit across is empty, when the last pit was not empty, when it is the opponent's pit, or when it is a store (free turn instead) (Task 2).
5. A label that leads to several places from one, followed by `FOLLOW`, is still refused (Task 1).

---

### Task 1: Labelled links

**Files:**
- Modify: `crates/engine/src/specs/game/map.rs` (`MapSpec`, `compile`, `RawLink`, `OneWayLink`, `compile_written`, a new `BetweenLink` and `LabelledLink`), `specs/game/board.rs` (writing a map back), `specs/game/hex_map.rs` (the `compile` call), `specs/game/game.rs` (the three `MAP_LABELS` checks), `logic/places/board/map.rs` (the tables)
- Create: `crates/engine/src/tests/link_labels/mod.rs`; `mod link_labels;` in `tests/mod.rs` (alphabetical)
- Modify tests that read `map.links` by `&'static str` keys: `tests/map_spec/mod.rs` (`every_label_has_a_list_for_every_place`), others as the compiler lists
- Modify docs: `specs/SPEC_FORMAT.md` (the Map boards section and error table)

**Interfaces:**
- Produces: `MapSpec.links: BTreeMap<String, Vec<Vec<usize>>>` (every built-in label present, plus each written label); `MapSpec.labelled: Vec<LabelledLink>` with `pub struct LabelledLink { pub label: String, pub from: String, pub to: String, pub both: bool }`; `MapSpec::compile(places, lines, one_way, labelled, source)`; `MapSpec::has_label(&self, label: &str) -> bool`; spec forms `{ "from": A, "to": B, "label": L }` and `{ "between": [A, B], "label": L }`.
- Consumes: existing `point` resolution and `Links::add` in `compile`.

- [ ] **Step 1: Write the failing tests** in `link_labels/mod.rs`, on a small inline map spec (three points `a`, `b`, `c` and a trivial `pass` action):
  - `a_written_label_is_a_link_of_the_map`: `{ "from": "a", "to": "b", "label": "across" }` gives `map.links["across"][a] == [b]` and `[b]` empty; `{ "between": ["a", "c"], "label": "across" }` gives both ways; `map.links["next"]` unchanged.
  - `a_labelled_link_is_no_line`: `map.lines` and `map.one_way` do not hold it; `map.labelled` does.
  - `a_move_an_around_and_a_follow_may_name_a_written_label`: a piece with `"step": "across"`, a test `AROUND ... "through": "across"` and a `FOLLOW ... "through": "across"` over a one-place-each label are read; the same spec without the link is refused with `UnknownLabel` naming `across`.
  - `a_follow_over_a_label_that_leads_to_several_places_is_refused` (`InvalidStep`).
  - `an_empty_a_long_or_a_derived_label_is_refused`: `""`, 65 bytes, and each of `ends`, `paths`, `corners`, `areas`, `sides` are `InvalidMap` naming the label; `next` is accepted and the link is then a `next` link (it appears in `one_way`).
  - `a_labelled_link_joins_points`: a path or an area as an end is refused (`InvalidMap`, "is not a point").
  - `a_labelled_link_round_trips`: parse, serialize (`serde_json::to_string`), parse again: the two `MapSpec`s are equal.
  - `an_unknown_key_on_a_labelled_link_is_refused`: `{ "from", "to", "label", "bogus" }` and `{ "between", "label", "bogus" }`.
  - `a_between_link_needs_a_label`: `{ "between": ["a", "b"] }` is refused.
- [ ] **Step 2: Run** `cargo test -p modern-chess-engine link_labels` → FAIL (unknown field `label`, unknown field `between`).
- [ ] **Step 3: Implement.** `MapSpec.links` keyed by `String`; `Links` keyed by `String` and seeded with `MAP_LABELS` plus each labelled link's label; `compile` takes `labelled: Vec<LabelledLink>`, validates each (label rules above; ends are points) and adds `links.add(label, from, to)` (and the reverse when `both`). `RawLink` becomes `Both([String; 2]) | OneWay(OneWayLink { from, to, label: Option<String> }) | Between(BetweenLink { between: [String; 2], label: String })`, both structs `deny_unknown_fields`; `compile_written` sends a link with no label or label `next` to `lines`/`one_way` and the others to `labelled`. `board.rs` writes `labelled` back as the object forms. `has_label` is true for the built-in labels and every key of `links`; replace the three `MAP_LABELS.contains` checks in `game.rs` with `map.has_label` (a game with no map keeps its grid message). `logic/places/board/map.rs` builds one table per key of `spec.links` and looks a name up by those keys. `hex_map.rs` passes `Vec::new()`.
- [ ] **Step 4: Run** `cargo test -p modern-chess-engine link_labels` → PASS; then the engine, session and wasm suites (fix the tests that read `map.links` keys by hand: only their key type changes). Update `SPEC_FORMAT.md`: the two new forms and the label rules in the Map boards section, the refusals in the error table.
- [ ] **Step 5: Commit** `feat(engine): maps take link labels of their own`.

---

### Task 2: The full Kalah

**Files:**
- Move: `crates/engine/src/tests/kalah/spec.json` → `crates/engine/specs/kalah.json` (`git mv`); update its readers: `tests/kalah/mod.rs` (two paths), `tests/unknown_fields/mod.rs` (two paths), `crates/session/tests/view.rs` (the `include_str!` path), `specs/SPEC_FORMAT.md` (the example's path)
- Modify: `crates/engine/specs/kalah.json` (links and the capture), `tests/kalah/mod.rs` (the capture tests, the header comment), `tests/catalogue_counts/mod.rs` (a row `kalah: N plies => N`)
- Create (if the skin carries over): `crates/game/pages/kalah.json`
- Modify docs: `specs/SPEC_FORMAT.md` (Kalah's annotated example, the Kalah limitation removed), `apps/docs/src` where Kalah is described

**Interfaces:**
- Consumes: Task 1's labelled links.
- Produces: `across` links `a1`↔`b6`, `a2`↔`b5`, `a3`↔`b4`, `a4`↔`b3`, `a5`↔`b2`, `a6`↔`b1` (`between`), and `store` links, one way, from each pit to the store of its row (`a1`…`a6` → `sa`, `b1`…`b6` → `sb`).

- [ ] **Step 1: Move the spec and fix its readers** (`git mv`, then the paths above); run the Kalah, unknown_fields and session `view` tests → PASS (the move changes nothing yet).
- [ ] **Step 2: Write the failing tests** in `tests/kalah/mod.rs`, positions set by hand with the existing `set`/`stones` helpers and the sowing played with `Choose` answers as the file's other tests do:
  - `the_last_stone_in_an_empty_pit_takes_the_pit_across`: SOUTH sows a pit whose last stone lands in an empty `a`-pit with stones in the `b` pit across; both the stones across and the last stone go to `sa`, the pit and the one across are empty, and the turn passes to NORTH.
  - `no_capture_when_the_pit_across_is_empty`: the last stone stays in its pit.
  - `no_capture_when_the_last_pit_was_not_empty`.
  - `no_capture_in_the_opponents_pit`: SOUTH's last stone lands in an empty `b` pit.
  - `north_captures_into_its_own_store`: the mirror, into `sb`.
  - `a_last_stone_in_the_store_is_still_a_free_turn_and_no_capture`.
  - `the_across_links_add_no_lines_to_the_view`: the session view of Kalah has only the `next` ring as lines (14 of them), not the six `across` links (in `crates/session/tests/view.rs`).
- [ ] **Step 3: Run** → the capture tests FAIL (no capture yet), the view test FAILS once the links exist or passes vacuously before: write the links first in Step 4 and watch it; if it cannot fail first, say so in the ledger.
- [ ] **Step 4: Write the spec.** Add the `across` and `store` links. In `SOW`'s `do`, after the free-turn `if`, add `{ "if": ALL[ IN_ZONE LAST in PITS; COUNT in AT LAST where STONE max 1; COUNT in AROUND of LAST through "across" where STONE min 1 ], "then": [ FOR_EACH in AROUND of LAST through "across": MOVE from EACH to LAST what STONE count ALL; FOR_EACH in AROUND of LAST through "store": MOVE from LAST to EACH what STONE count ALL ] }`. The capture moves the stones across onto the last pit, then every stone of the pit to the store. If a block refuses (a `FOR_EACH` over `AROUND`, or `LAST` after the first inner `MOVE`), rule on the smallest fix that keeps to existing blocks and ledger it; a new block is not allowed.
- [ ] **Step 5: Run** the Kalah tests → PASS; add the `kalah` row to `catalogue_counts` (record N from the first run; check that the table's alphabetical order is kept) and run the whole engine, session and wasm suites. Update the Kalah header comment (no longer "the capture is not here").
- [ ] **Step 6: The page.** Try `crates/game/pages/kalah.json` from Pong hau k'i's page (a map skin). If the skin does not carry over, leave the page out and say so in the final report; do not guess a skin.
- [ ] **Step 7: Docs.** Kalah's annotated example in `SPEC_FORMAT.md` shows the links and the capture; the "Kalah has no capture of the opposite pit" limitation is removed; `apps/docs/src` mentions of the missing capture are updated.
- [ ] **Step 8: Commit** `feat(engine): the full Kalah, with its capture, in the catalogue`.

---

### Final

- [ ] Roadmap: `docs/superpowers/specs/2026-11-23-remaining-work.md` (item 2 done; per-player links and sliding noted as not built, waiting for a game); `cargo fmt --check`, clippy count, whole suites; final whole-branch review by an Opus subagent (ledger rulings in its brief); report rulings, deferred minors, and what needs Frank's database run.
