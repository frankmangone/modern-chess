# The spec builder, slice 1: foundation and the generated games

**Goal:** A TypeScript package, `tools/spec-builder`, that builds Snakes and Ladders, Backgammon and Kalah (specs and pages) from typed block helpers and recipes, byte-identical to the committed JSON, with a drift check that keeps it so. The four Python generators are then deleted.

**Spec:** `docs/superpowers/specs/2026-11-29-spec-builder-design.md` (decisions: one-way, recipes are code, audience is us and agents, foundation plus the generated games).

## Constraints

- One-way. A recipe expands to plain blocks and only the blocks are saved. The engine, the server, the saved format and the wasm do not change and never hear of recipes.
- Byte-identical. `specs/*.json` and `crates/game/pages/*.json` stay the source of truth, committed and loaded by the Rust tests as now. The builder reproduces them exactly, so every generated file's diff in this slice is empty (Kalah's once-off formatting aside, in its own commit).
- Recipes only write blocks. They do not call the engine. `build` alone does, to validate what it built.
- Node 24 or newer runs the TypeScript directly (types are stripped; no bundler, no build step). Tests use `node:test`. The one dev dependency is `typescript`, for `tsc --noEmit`.
- Key order matters for byte-identity, so every helper writes keys in the order the committed files have them.
- A recipe is not done until it reproduces a real game. Each task ends with the drift check passing for the game it covers.
- Commits are conventional and end with `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. Do not push or open a PR without being asked.

## Findings that shaped the plan

- The four Python generators reproduce the committed files exactly today (run, `git status` shows no change), so "byte-identical" is a real target, not an aspiration.
- Python's `json.dumps(indent=2)` and `JSON.stringify(x, null, 2)` agree for what these files hold (integers, ASCII strings), so no custom serialiser is needed. The art uses Python's `f"{v:.3f}"`, which rounds an exact tie to even where `toFixed` rounds up; a small `num` helper does it Python's way.
- `kalah.json` is hand-written and its formatting is not canonical (`"at": [1, 0]` on one line). It is normalised once, in its own commit, so the diff is visible and the rest of the slice is then byte-identical.
- The engine's `Board` constructor in the wasm package refuses a bad spec, so `build` validates through the same code that plays it: `apps/docs/theme/pkg` (made by `make wasm`), loaded in Node with `initSync`.

## Layout

```
tools/spec-builder/
  package.json  tsconfig.json  README.md
  src/
    blocks.ts           typed constructors: tests, regions, effects, choices; fail when written
    num.ts              Python-style number formatting for drawn art
    scaffold.ts         players, turns, pieces, bags, zones, end checks, page sections
    build.ts            build(game) -> { spec, page }; validates the spec through the engine
    engine.ts           loads the wasm engine in Node
    recipes/            track.ts (numbered track, jump links), dice.ts, sowing.ts, backgammon.ts
    art/                trackArt.ts, backgammonArt.ts, piece faces
    games/              snakes-and-ladders.ts, backgammon.ts, kalah.ts, index.ts
    generate.ts         writes every game, or with --check fails when a file differs
  test/                 node:test files
```

## Tasks

1. **Kalah formatting.** Rewrite `crates/engine/specs/kalah.json` with canonical two-space formatting. Prove it is the same document (parsed equal) and the Rust suites pass. Own commit.
2. **Skeleton.** Package, tsconfig, `num`, block helpers with tests (each helper produces the exact keys and order, and refuses a malformed block).
3. **Build, engine, drift check.** `build(game)`, the wasm loader, `generate.ts` with `--check`, Makefile target. Proved on Snakes and Ladders first: the track, jump links, dice-into-bag and walk recipes, the track art, and the page.
4. **Backgammon.** Per-die moves, hit, bar, bear-off and the board art, the page.
5. **Kalah.** Holders ring, sowing, capture across, store; the page.
6. **Retire the generators.** Delete the four Python files, point `apps/docs/README.md` and any doc that names them at the package, write `tools/spec-builder/README.md`.

## Review focus

1. Every generated file is byte-identical, including Backgammon's 300 KB (the check compares bytes, not parsed JSON).
2. A malformed block fails at the helper that wrote it, with a message naming it, before the engine sees it.
3. `build` refuses a spec the engine refuses, and the message is the engine's.
4. Nothing outside `tools/`, the Makefile, docs and `kalah.json`'s whitespace changed.

## Open, decided here

- *May a recipe ask the engine a question while building?* No. Recipes only write blocks.
- *Where do recipe parameter types get documented?* In a doc comment on the function, in plain words, one `@param` per option. The book can link to them later.
