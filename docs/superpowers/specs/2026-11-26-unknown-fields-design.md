# Unknown fields refused everywhere

Item 3 of `2026-11-23-remaining-work.md`. A short design: a small change.

## Intent

A key a spec writes that the format does not have must be refused, wherever it is written. Today a
misspelled `"onyl"` or `"turn"` is read as nothing, and the spec plays with the rule missing. That
is the anti-goal of the whole route: a spec that silently means something other than what it
says. The worst place is the top level, where a whole rule (`"end_check"` for `"end_checks"`) can
vanish.

Frank's rulings: option 3 (no hints for old keys, plain errors), and **every spec must comply with
the current format, with no transitional escape**: the old-forms machinery goes entirely, not
only the key placeholders.

## What is true today

Strict already (`deny_unknown_fields`): tests, regions, places and filters in tests, actions,
choices, effects, `if`, end checks and their scores, bags, zones, stages, `every_action`, one-way
links (since #82). So "effects as strict as the test blocks" is already done.

Not strict, and the whole job: `GameSpec` (top level), `BoardSpec`/`RawBoardSpec`, `RawPlace` (map
places), `PlayerSpec`, `PiecePositionSpec`, `PieceSpec`, `MoveSpec`, `RepeatSpec`, `TurnSpec`.

Old-form machinery that exists only to refuse old text with a pointer to the new one, all of it
going:

- `old_forms.rs` whole: the `OldField` type, 22 `old_*` placeholder fields (13 on the game, 5 on pieces, 4 on moves), their
  readers and the `refuse_field!` macro, the `until` reader (an `until` written as a
  name) and the `condition` reader (a test written as a named condition).
- `GameSpecError::OldForm` and `old_form_refusal` in the parser.
- The `END_TURN` effect, kept in `EffectSpec` only to be refused (and mapped in `steps/build.rs`).

## Goals

- Every spec type that reads a JSON object refuses a key it does not know, with serde's
  `unknown field` error, which already lists the keys it expected.
- A form the format no longer has is just an unknown key, an unknown variant or a wrong type, with
  serde's own message. No special cases remain for "used to be valid".

## Non-goals and anti-goals

- No table of old keys and hints (option 2, not chosen).
- Not a change to what any valid spec means. Every catalogue, book and fixture spec must read
  exactly as before; a spec that was only accepted because a key was ignored is a bug in that
  spec and gets fixed there, not excused.
- Specs already stored in a database under an old form fail to load, as accepted since #77.

## Approaches

1. Add `deny_unknown_fields` and keep the placeholders. Smallest, but leaves the type noise.
2. Retire the placeholders, keep the hints in one table that upgrades an unknown-field error for
   a known old key. Keeps migration help for agents, at the cost of a table to maintain.
3. **Retire everything old-form, plain errors. Chosen by Frank**: stricter, and tighter code;
   nothing is in production.

## The design

1. `#[serde(deny_unknown_fields)]` on each of the nine types. Where a type is read through a
   `from` reader (`BoardSpec` via `RawBoardSpec`) or is a raw type serde reads directly
   (`RawPlace`), the denial goes on the type serde reads. It cannot be used with `flatten`.
2. Delete `old_forms.rs`, the `OldForm` variant and its parser hook, and the `EndTurn` effect.
   A repeat's `until` becomes a plain `Option<TestSpec>` (`null` still reads as none).
3. Tests, red before the change:
   - one table-driven test that adds `"bogus"` at each of the nine places in a real spec and
     expects a refusal naming the key;
   - one test listing the retired keys (the twenty-two, and `END_TURN`, `until` as a name,
     `condition`) and expecting each to be refused, so they cannot come back by accident;
   - the existing tests that matched `OldForm` or asserted a message naming the replacement
     (go, test blocks, action blocks, every-move rules, old end forms, bags, stages) assert only
     that the form is refused. The `old_end_forms` test module folds into the retired-keys test.
4. Docs: the "Old forms" section of `SPEC_FORMAT.md` becomes "Retired forms": same table of what
   to write instead, but no claim of an `OldForm` error (the error table in chapter 11 and the
   lines at SPEC_FORMAT 928-930 and 1251 lose it). It stays as migration help for people.

## Left to the builder

Which catalogue, book or fixture specs rely on an ignored key (found by running the whole suite,
the docs build and the web tests; fix the spec). Whether `RawBoardSpec` allows the denial
directly or needs the check moved. Names of the test helpers.

## Calls I made (for you to check)

- The "Retired forms" table stays in the docs as migration help, with plain-error wording. If you
  want it gone with the machinery, say so; it is only prose.
- Retired keys get no check beyond "refused": tests no longer assert what the message names.
- Date: `2026-11-26`, to sort after the turn-stages design, as the pass's other docs do.
