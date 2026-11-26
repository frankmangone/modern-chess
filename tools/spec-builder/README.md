# The spec builder

Builds a game's **spec** (the blocks the engine reads) and its **page** (what the hub shows) from
typed helpers and recipes, instead of writing the JSON by hand or in a one-off script. Snakes and
Ladders, Backgammon and Kalah are built this way; the Python generators they had are gone.

It is a library for us and for agents. It is not an editor, and the engine, the server and the saved
format never hear of it: **a recipe expands to plain blocks, and only the blocks are saved.** The
committed JSON (`crates/engine/specs/*.json`, `crates/game/pages/*.json`) stays the source of truth,
and the Rust tests load it as they always did.

Design: `docs/superpowers/specs/2026-11-29-spec-builder-design.md`. First slice:
`docs/superpowers/plans/2026-11-29-spec-builder-slice-1.md`.

## Run it

Needs **Node 24 or newer** (it runs the TypeScript directly: no build step, no bundler) and the
engine's WebAssembly, which `make wasm` builds.

```sh
make specs-check   # rebuild every game and fail if a file differs from the committed one; run the tests
make specs         # rewrite the files after you changed a recipe
```

Without make: `node tools/spec-builder/src/generate.ts [--check] [slug ...]`, and, inside
`tools/spec-builder`, `pnpm install` once, then `pnpm test` and `pnpm typecheck`.

`--check` compares **bytes**, not parsed JSON, so key order and formatting count. That is the point:
a recipe is done when it reproduces a real game exactly.

## What is in it

```
src/blocks.ts     typed constructors: regions, tests, effects, choices, conditions, actions
src/scaffold.ts   map boards, players, turns, pieces, bags, zones, end checks, page sections
src/recipes/      track (numbered track, jump links), dice, race, backgammon, sowing
src/art/          drawn boards and piece faces (SVG strings)
src/games/        one file per game: recipes plus its page; index.ts lists them
src/build.ts      build(game) -> { spec, page }, read through the engine
src/engine.ts     the engine in Node (the wasm in apps/docs/theme/pkg)
src/generate.ts   writes every game, or with --check compares
```

- **Helpers refuse a malformed block when it is written**, naming themselves (`count: min 2 is
  above max 1`), not when the engine reads the spec. They follow `specs/SPEC_FORMAT.md`; the Rust
  spec types stay the authority.
- **`build` asks the engine.** The builder has no second opinion on what is legal: a spec the
  engine refuses is refused with the engine's own message.
- **Every helper writes its keys in the order the committed files have them.** `JSON.stringify`
  keeps insertion order, so that order is what makes the output byte-identical.

## Add a game

Write `src/games/<game>.ts` exporting a `Game` (`spec`, `page`, `files`), add it to
`src/games/index.ts`, and run `make specs`. Reuse a recipe where one fits (a map board with a track
and jumps; two dice in a bag; sowing; a race with hitting); write a new one in `src/recipes/` when
none does, as a function that takes options and returns blocks. Document each option in a doc
comment on the function, in plain words.

A recipe only writes blocks: it never asks the engine anything. A new recipe is proved the way these
were: build a real game with it and compare with the file, and compare the blocks it writes with the
ones the committed spec has (see `test/backgammon.test.ts`).

## Not here yet

Chess-family recipes (pieces, castling, en passant, promotion, check), a command line (`explain`,
`check` on its own), and the visual editor for the hub. They are the later slices of the design.
