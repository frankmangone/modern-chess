# The hand, and where removed pieces go: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (native, as Frank asked for slice work: judgment-heavy, tightly coupled tasks). Steps use checkbox syntax.

**Goal:** `hand: { enabled }` replaces `hand_enabled`; `removed_to: { "hand": "MOVER" }` on `MOVE`/`REMOVE` replaces the implicit capture-to-hand; `captured_as` replaces `demotes_to`. Shogi and Crazyhouse play exactly as before.

**Architecture:** the spec reader gains the new forms and refuses the old; the compiled effects carry a `removed_to` flag; a running way collects the codes it sends to the mover's hand (`Way.to_hand`); playing a way applies `from_hand` and `to_hand`; `take_into_hand` is deleted. Saved state is unchanged.

**Spec:** `docs/superpowers/specs/2026-11-23-hand-effects-design.md`

## Global Constraints

- Every catalogue game plays exactly as before: `catalogue_counts` is unchanged.
- Speed: within 5% of `main` per game (`crates/engine/examples/speed.rs`, `ab.sh`).
- Reading a spec refuses what it does not know: `removed_to` without `hand.enabled`, and any `removed_to` value but `{ "hand": "MOVER" }`.
- Old forms are refused with the `old_forms` pattern: `hand_enabled`, `demotes_to`.
- Clippy clean for the engine (`indexing_slicing`, `expect_used` denied outside tests).

## Review Focus

- A hand-enabled game whose effect has no `removed_to`: nothing enters the hand (test in Task 2).
- `removed_to` on a removal of the mover's own piece: nothing enters the hand (Task 2).
- `captured_as` naming a code with no piece: refused when read (Task 1).
- A saved Shogi game from before the change still loads and plays (Task 3).

---

### Task 1: The spec reads the new forms and refuses the old

**Files:** `src/specs/game/game.rs`, `src/specs/game/action.rs`, `src/specs/game/piece/piece.rs`, `src/specs/game/old_forms.rs`, `src/specs/game/tests.rs`.

**Interfaces — produces:** `GameSpec.hand: HandSpec` (`#[serde(default)] struct HandSpec { enabled: bool }`, `deny_unknown_fields`); `PieceSpec.captured_as: Option<String>`; `EffectSpec::Move { from, to, removed_to: Option<RemovedTo> }` and `EffectSpec::Remove { at, removed_to: Option<RemovedTo> }` where `RemovedTo { hand: HandOf }` and `enum HandOf { Mover }` (a value `OWNER` is refused with the text "comes with 4c-2").

- [ ] Tests (in `specs/game/tests.rs`): `hand` reads; `hand_enabled` refused naming `hand`; `demotes_to` refused naming `captured_as`; `removed_to` without `hand.enabled` refused; `removed_to` with `OWNER` refused; `captured_as` of an unknown code refused. Watch them fail.
- [ ] Implement; run `cargo test -p engine specs::`.
- [ ] Commit.

### Task 2: A way sends removed pieces to the hand

**Files:** `src/logic/blueprint/steps/{mod.rs,build.rs}`, `src/logic/game/ways/{mod.rs,run.rs}`, `src/logic/game/mod.rs` (`rules.demotes_to` → `rules.captured_as`), `src/logic/game/handlers/choose.rs`.

**Interfaces — produces:** `EffectStep::Move { from, to, removed_to: bool }`, `EffectStep::Remove { at, removed_to: bool }`; `Way.to_hand: Vec<Symbol>`.

- [ ] Tests (a small spec in `src/tests/hand_effects/`): a capture with `removed_to` puts the enemy piece, as `captured_as`, in the mover's hand; the same capture without it puts nothing; removing the mover's own piece with it puts nothing; a way lists `to_hand`. Watch them fail.
- [ ] `Run.remove` takes `to_hand: bool`; when set and the piece is another player's, push its `captured_as` code to `run.to_hand`; `Way.to_hand` carries it.
- [ ] `play_way` applies `way.to_hand`; delete `take_into_hand` and the `hand_enabled` guard around it.
- [ ] Run the engine suite. Commit.

### Task 3: The catalogue, the docs, the proofs

**Files:** `specs/shogi.json`, `specs/crazyhouse.json`, `src/tests/**` specs naming `hand_enabled`/`demotes_to`, `apps/docs/src/07-the-hand.md`, `11-design-your-own.md`, the reference, the route doc.

- [ ] A script rewrites the catalogue and test specs: `hand_enabled: true` → `hand: { enabled: true }`, `demotes_to` → `captured_as`, `removed_to` on every effect that removes an enemy piece.
- [ ] `catalogue_counts`, Shogi and Crazyhouse tests, and a load of a stored pre-change Shogi save pass unchanged.
- [ ] Docs rewritten (chapter 7, reference, design-your-own); the route doc and the generalisation doc record the split and "built".
- [ ] Speed gate (`ab.sh`), opus whole-branch review, ONE fix pass, PR.
