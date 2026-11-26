# The spec builder and recipes: design

Status: brainstormed with Frank on 2026-11-29; slice 1 built (`2026-11-29-spec-builder-slice-1.md`). This is the "recipes and the builder"
item of `2026-11-23-remaining-work.md` section 6.

## Why now

The rule-blocks pass (2026-11-22) chose a uniform block tree as the only vocabulary and said
that familiar patterns would come back as *recipes in a builder*, not as shorthand in the
format. Specs have since grown: Backgammon is about 300 KB, Snakes and Ladders 32 KB, and both
are already written by Python generators (`tools/gen_*_spec.py`) because nobody writes that by
hand. Those generators are recipes in all but name, in a language the rest of the repo does not
use and with helpers (`count`, `at_bag`, `has_die`, `hit`) that each one re-invents.

## Decisions (Frank, 2026-11-29)

- **Audience: us and agents first.** A library and a command line, not a visual editor. The
  editor for the hub's user-proposed games comes later and sits on this library; it is not
  designed here.
- **One-way.** A recipe expands into plain blocks and only the blocks are saved. The engine, the
  server, the saved format and the wasm never hear about recipes. No round-trip field.
- **Recipes are code.** Typed functions with parameters that return blocks. Data templates
  (JSON with slots) are not ruled out later for the simple ones, but they are not the
  foundation.
- **First slice: foundation plus the generated games.** Smallest slice that shows the idea works.

## Shape

A TypeScript package under `tools/spec-builder` with three layers.

1. **Block helpers.** Typed constructors for the blocks the generators write by hand today:
   tests (`count`, `inZone`, `some`, `all`, `not`), regions (`atBag`, `atPlace`, `around`),
   effects (`move`, `remove`, `place`, `forEach`, `stage`), choices. A malformed block fails
   when it is written, not when the engine reads the spec. The types follow
   `specs/SPEC_FORMAT.md`; the Rust spec types stay the authority.
2. **Recipes.** Functions that return blocks or parts of a spec. Grouped as:
   - *Boards*: grid, hex, map (named places, links with labels, zones from ranges).
   - *Movement sets*: the standard pieces' moves, a leaper, a slider.
   - *Rules*: sowing, hit-and-bar, bear-off, jump links (snakes, ladders), dice rolled into a
     bag, custodial and liberty capture, castling, en passant, promotion, forced capture,
     drop from hand.
   - *Scaffold*: players, turns, stages, end checks, bags.
3. **`build(game)`.** Takes a game described as recipes plus its page (slug, tags, skin, how-to
   text, board art) and returns two plain objects: the spec and the page. Pages are built here
   too, so the Python page generators go with the spec ones.

Run with Node 24, which strips TypeScript types itself: no bundler and no build step for the
tool. Tests use `node:test`.

## Validation and drift

- **The real reader is the only judge.** `build` loads the result through the engine, so the
  builder has no second opinion on what is legal. In Node that means the wasm package
  (`apps/docs/build-engine.sh`); the Rust catalogue tests already load every committed spec.
- **The committed JSON stays the source of truth.** `specs/*.json` and the page JSON are
  generated, checked in, and loaded by the Rust tests as now. A check (a Makefile target and a
  `node --test`) rebuilds every game and fails when the output differs from the committed file.
  That is today's byte-identical check, made permanent.

## First slice

- Package skeleton, block helpers, `build`, the drift check.
- Recipes needed by three games: dice into a bag, jump links, a numbered track (Snakes and
  Ladders); sowing and holders (Kalah, which today is hand-written JSON); per-die moves, hit,
  bar, bear-off and the board art (Backgammon).
- Rebuild those three specs and their pages **byte-identical** to the committed files, then
  delete the four Python generators. A recipe is not done until it reproduces a real game.
- Docs: a short `tools/spec-builder/README.md`; `apps/docs/README.md` points to it.

Not in the slice: chess-family recipes, a command line, a visual editor, data-template
recipes.

## Later slices

1. **Chess-family recipes**: pieces, castling, en passant, promotion, check and checkmate,
   checked against `chess.json` and `shatranj.json` byte-identical (or, where the committed
   file was hand-tuned, equal after normalising key order, said plainly).
2. **Command line**: `build`, `check` and `explain`. `explain` lists the blocks a spec uses
   ("dice, stages, holders"), which is the `requires(spec)` idea from the per-game-builds note
   and is useful to the book and the hub on its own.
3. **The hub editor**: a form over the same recipes with a live board and a play-test.

## Open for the plan

- Whether a recipe may call the wasm engine to ask a question while building (for example "is
  this place reachable"). Leaning no: recipes only write blocks.
- Where recipe parameter types get their documentation: next to the function, and the book
  linking to it.
- Kalah's committed JSON may have key order or spacing that a builder would not reproduce; if
  so the slice normalises it once, in its own commit, so the diff is visible.
