# Spec versioning

> Amended on Frank's ruling before the build: the field is **mandatory** (the design first made
> it optional), and the stored games in the database get the key from a migration.

Closing task of the design run. A short design: a small change, one optional top-level field, one
refusal, one probe in the parser.

## Intent

A spec states which version of the spec format it is written for. What exists today is
**version 1**. When the format later changes what an existing spec means (or drops a form), the
engine can then tell "written for a newer format" from "misspelled", and refuse the first with a
message that says so, instead of a wall of `unknown field` errors or, worse, reading the spec as
something it does not say (the anti-goal of the whole route).

## What is true today

- Every spec is read through `parse_spec_str` (`specs/game/parser.rs`): the session, the wasm
  build, the game crate (`domain/spec.rs`, through `parse_rules`) and the examples all go through
  it. One place sees every spec.
- `GameSpec` refuses unknown keys (`deny_unknown_fields`). A spec written for a newer format that
  adds a key would be refused with a message that names only that key: true, but unhelpful.
- The game crate stores each game's rules as JSON in a database row that has its own `version`
  (the revision of that game's rules, bumped when the rules change). That is a different thing.

## Goals

- A top-level `"format_version": 1` is required. Every catalogue, book, test and fixture spec
  states it, and a migration gives it to the rules already stored in the database.
- Any other value is refused with an error that names the version found and the one this engine
  reads, *before* the rest of the spec is read, so a newer spec gets this message and not a
  parade of unknown-field errors.
- The documentation says what the number means and when it will change.

## Non-goals and anti-goals

- No upgrading or migration of specs between versions, and no reading of more than one version:
  there is one version. (When version 2 exists, the engine decides then whether to read both.)
- Anti-goal: a version that is accepted and ignored. `"format_version": 2` must be refused, as
  must `0`, `-1`, `1.5`, `"1"`, `null` and a missing key, not read as 1.
- No change to what any spec means.

## Approaches

1. **Required field, checked in a probe before the full read. Chosen by Frank.** Every spec
   says what it is written for, so there is no "absent means 1" to carry into version 2. Costs a
   one-time edit of every spec (done by script) and a database migration.
2. Optional field, absent meaning 1. Smaller (no edits), proposed first. Rejected by Frank.
3. Required field checked only in `validate`. Simplest code, but a newer spec fails in serde
   first, on its unknown keys, with the wrong message. That defeats the point. Rejected.

## The design

1. `GameSpec.format_version: u32`, required, written back as it is. `pub const FORMAT_VERSION:
   u32 = 1`, exported from `specs`. The field is known to serde so `deny_unknown_fields` accepts it.
2. `parse_spec_str` first reads the text as a JSON value. If it is an object, the key must be
   present and be the whole number `FORMAT_VERSION`, else the new errors are returned and nothing
   else is read. If the text is not an object or not JSON, the check is skipped and the normal read
   gives its normal error.
3. New `GameSpecError::MissingFormatVersion` and `UnsupportedFormatVersion(String)` (the value as
   written: `2`, `"1"`, `null`). `validate` also checks the field, so a `GameSpec` built in Rust
   with another value is refused too.
4. Every spec states `"format_version": 1`: the catalogue, the book's examples, the test and
   fixture specs, and the specs inside Rust and docs snippets (inserted by script).
5. The database: migration `0006_format_version` adds the key to the `rules` of every stored
   version (down: removes it), as migration 0005 rewrote stored commands. `rules_hash` is left as it
   was, so the next seeding stores the same rules once more as a new version; matches in progress
   keep the migrated old one.
6. Tests, red first: `1` reads; a missing key, `2`, `0`, `-1`, `1.5`, `"1"`, `null`, `true` are
   refused with the right error; a version-2 spec with keys version 1 lacks gets the version error;
   written back it keeps the key; a Rust-built `GameSpec` with 2 is refused by `validate_specs`;
   the session refuses both (wasm and the game server go through the same parser); the migration
   adds the key, leaves rows that have it alone and its down removes it (needs the database).
7. Docs: `SPEC_FORMAT.md` (field, a "Versions" section with what bumps the version and what does
   not, error rows), book chapters 1 and 11, the roadmap.

## Left to the builder

The exact message wording; how the check reads the text.

## Calls I made (for you to check)

- The key is `format_version`, not `version`, since a stored game already has a `version` (the
  revision of its rules) and the two would be confused when read side by side.
- The policy for what bumps the version is written in the docs now, so the first breaking
  change has a rule to follow.
- The migration leaves `rules_hash` stale (SQL cannot reproduce the Rust hash), at the price of one
  duplicate version per game at the next seeding.
- The session shows errors as debug text (`UnsupportedFormatVersion("2")`), as for every error;
  not changed here.
