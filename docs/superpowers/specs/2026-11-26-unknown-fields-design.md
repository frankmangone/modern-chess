# Unknown fields refused everywhere

Item 3 of `2026-11-23-remaining-work.md`. A short design: a small change.

## Intent

A key a spec writes that the format does not have must be refused, wherever it is written. Today a
misspelled `"onyl"` or `"turn"` is read as nothing, and the spec plays with the rule missing. That
is the anti-goal of the whole route: a spec that silently means something other than what it
says. The worst place is the top level, where a whole rule (`"end_check"` for `"end_checks"`) can
vanish.

## What is true today

Strict already (`deny_unknown_fields`): tests, regions, places and filters in tests, actions,
choices, effects, `if`, end checks and their scores, bags, zones, stages, `every_action`, one-way
links (since #82). So "effects as strict as the test blocks" is already done.

Not strict, and the whole job: `GameSpec` (top level), `BoardSpec`/`RawBoardSpec`, `RawPlace` (map
places), `PlayerSpec`, `PiecePositionSpec`, `PieceSpec`, `MoveSpec`, `RepeatSpec`, `TurnSpec`.

Nineteen placeholder fields (`old_*: OldField`, on the game, pieces and moves) exist only to catch
an old key and refuse it with a message naming what replaced it. Everything else in the format
that unknown-field denial cannot catch stays as it is: old forms written as a *value* under a key
the format still has (`until` as a name, `condition` in a test, the `END_TURN` effect, old map
and end forms).

## Goals

- Every spec type that reads a JSON object refuses a key it does not know, with serde's
  `unknown field` error, which already lists the keys it expected.
- The `OldField` placeholders, their reader functions in `old_forms.rs`, and the macro that
  defines them are deleted. A removed key is now just an unknown key.

## Non-goals and anti-goals

- No table of old keys and hints (the option not chosen, below): an old key gets the plain error.
- Not a change to what any valid spec means. Every catalogue, book and fixture spec must read
  exactly as before; a spec that was only accepted because a key was ignored is a bug in that
  spec and gets fixed there.
- The `OldForm` error and the value-level refusals above stay: they are not placeholders.

## Approaches

1. Add `deny_unknown_fields` and keep the placeholders. Smallest, but leaves the type noise.
2. Retire the placeholders, keep the hints in one table that upgrades an unknown-field error for
   a known old key. Keeps migration help for agents, at the cost of a table to maintain.
3. **Retire the placeholders, plain errors. Chosen by Frank**: stricter, and tighter code; nothing
   is in production and the format's "Old forms" table in `SPEC_FORMAT.md` still says what to write
   instead.

## The design

1. `#[serde(deny_unknown_fields)]` on each of the nine types. Where a type is read through a
   hand-written or `from` reader (`BoardSpec` via `RawBoardSpec`, `RawPlace`), the denial goes on
   the raw type that serde reads. Not possible with `flatten` or inside untagged enums; the one
   untagged enum that holds objects (`RawLink`) is already handled in #82.
2. Delete the nineteen `old_*` fields and the readers in `old_forms.rs` that only served them
   (`top_level_conditions`, `drop_restrictions`, `restrictions`, and the `refuse_field!` ones).
   Keep `until`, the `condition` test reader and `old_form_refusal`.
3. Tests: a table-driven test that adds `"bogus"` at each of the nine places in a real spec and
   expects a refusal naming the key, red before the change. The tests that asserted an old key is
   refused *naming its replacement* (`jump_enabled`, `hand`/`hand_enabled`, the end-check old keys,
   `drop_restrictions`...) now assert only that the key is refused, naming the key itself.
4. Docs: the "Old forms" section of `SPEC_FORMAT.md` and the error table in chapter 11 say an old
   key is refused as an unknown field (no longer an `OldForm` error naming the replacement);
   the "write instead" column stays as migration help for people. `known-limitations.md` loses
   nothing.

## Left to the builder

Which of the nine types have places in the catalogue or book specs that rely on an ignored key
(found by running the whole suite and the docs build; fix the spec, not the rule). Naming of the
test helper. Whether `RawBoardSpec`'s serde attributes allow the denial directly or need the
check moved.

## Calls I made (for you to check)

- The value-level old-form refusals stay; only the key placeholders go. If you meant "retire the
  whole old-forms machinery", say so: it is a larger cut, and some of those messages (END_TURN,
  `until` as a name) are tested for naming their replacement.
- The `OldForm` error variant stays, since the above still produce it.
- Date: `2026-11-26`, to sort after the turn-stages design, as the pass's other docs do.
