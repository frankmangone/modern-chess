# Spec versioning

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

- A top-level `"format_version": 1` is read. Absent means 1, so no existing spec, page, test or
  stored row changes.
- Any other value is refused with an error that names the version found and the one this engine
  reads, *before* the rest of the spec is read, so a newer spec gets this message and not a
  parade of unknown-field errors.
- The documentation says what the number means and when it will change.

## Non-goals and anti-goals

- No upgrading or migration of specs between versions, and no reading of more than one version:
  there is one version. (When version 2 exists, the engine decides then whether to read both.)
- No field made required: that would touch every catalogue, book and fixture spec for nothing.
- Anti-goal: a version that is accepted and ignored. `"format_version": 2` must be refused, as
  must `0`, `-1`, `1.5`, `"1"` and `null`, not read as 1.
- No change to the database: stored rules keep their JSON as written (no field is added to
  them, and their hash does not change).

## Approaches

1. **Optional field, checked in a probe before the full read. Chosen.** Smallest; gives the
   newer-spec message; nothing existing changes.
2. Required field. Stricter, forces every spec to say it, but touches every spec in the
   catalogue, book, tests and the database, and the benefit (certainty about the version) is the
   same once the writer states it. Rejected for the churn.
3. Optional field checked only in `validate`. Simplest code, but a newer spec fails in serde
   first, on its unknown keys, with the wrong message. That defeats the point. Rejected.

## The design

1. `GameSpec.format_version: u32` (`#[serde(default = ...)]`, skipped when written back if it is
   the current one). `pub const FORMAT_VERSION: u32 = 1`, exported from `specs`. The field is
   known to serde so `deny_unknown_fields` accepts it.
2. `parse_spec_str` first reads the text into a probe that has only `format_version` (as a raw
   JSON value, unknown keys ignored). If the key is present and is not the whole number
   `FORMAT_VERSION`, it returns the new error and goes no further. If the text is not an object or
   not JSON, the probe is skipped and the normal read gives its normal error.
3. New `GameSpecError::UnsupportedFormatVersion(String)` carrying the value as written, with the
   message: `This engine reads format_version 1; the spec says <value>`. `validate` also checks
   the field, so a `GameSpec` built in Rust with another value is refused too.
4. Tests, red first: absent reads as 1; `1` reads; `2`, `0`, `-1`, `1.5`, `"1"`, `null` are
   refused with the new error; a version-2 spec that also has a key version 1 does not have gets
   the version error, not the unknown-field one; a spec written back (`serde_json::to_string`)
   has no `format_version`, and one that states it reads the same as one that does not; a
   `GameSpec` built in Rust with 2 is refused by `validate_specs`; the session and wasm refuse
   it too (one test each: they go through the same parser, so this pins the route).
5. Docs: `SPEC_FORMAT.md` gets the field in the top-level table, a short "Versions" section
   (what 1 is; **what bumps the version**: a change to what an existing spec means, or removing
   a form that was valid; **what does not**: adding a block, a field or a form, which version 1
   specs keep reading as before) and a row in the error table. Chapter 11 of the book gets a
   line telling authors to state it. The remaining-work roadmap marks versioning done.

## Left to the builder

The exact message wording; whether the probe is a struct or `serde_json::Value`; whether the
catalogue's own specs state `"format_version": 1` (my call: no, left as they are; the book's
chapter 11 recommends writing it in new specs).

## Calls I made (for you to check)

- The key is `format_version`, not `version`, since a stored game already has a `version` (the
  revision of its rules) and the two would be confused when read side by side.
- Optional, not required (as you agreed earlier), and the existing specs are not edited.
- The policy for what bumps the version is written in the docs now, so the first breaking
  change has a rule to follow.
