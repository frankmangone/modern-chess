# The rules engine in a sandbox

Status: built (branch `feature/engine-sandbox`). Settled with Frank on 2026-11-27: running a stranger's
rules on the server needs a hard budget on every move of every game, built **before** publishing
(`2026-11-27-builder-publish-design.md`), not after.

## The problem

The server used to call the engine from the HTTP handler, with `catch_unwind` for a panic and nothing
for anything else. Rules written by a stranger can:

- never finish (a `repeat` that does not stop): a handler thread is held for good, and a few stop the server;
- ask for more memory than the machine has: the system kills the whole server;
- overflow the stack: that is an abort, which `catch_unwind` cannot catch;
- grow a process-wide table (the engine interns every position name in a table that only grows), so
  publishing names forever is a slow leak.

Written by the hub's own team, the rules never did any of this. User rules will.

## What was built

The engine is compiled to WebAssembly and run with `wasmtime`. Nothing else on the server runs a game.

| Crate | Does |
|-------|------|
| `sandbox-protocol` | The questions and answers (`Request`, `Response`) and `handle`, the one function that answers them. Pure data in, data out. |
| `sandbox-guest` | The engine as a module with **no imports** (no clock, files, network or random numbers) and two exports, `alloc` and `handle`. |
| `sandbox` | The host: compiles the module once, and for **every call** makes a fresh instance with a **fuel budget** and a **memory cap**. |
| `game` (`Engine`) | The server's only door to the rules. Remembers per rules' hash what only the rules decide (a game's seats, the picture of its board). |

- **Fuel** counts instructions, so "too long" is the same on every machine. A call that runs out traps
  and the caller answers with the fixed line ("the game engine could not handle this position"); the
  cause goes to the log. The budget is 2 billion, more than twenty times what the heaviest catalogue
  call costs (backgammon, 84 million); `crates/sandbox/tests/sandbox.rs` prints the table and fails if
  any call grows past a twentieth of the budget.
- **Memory** is capped (128 MiB), the stack overflows into a trap, and **nothing is kept between calls**,
  so the position-name table starts empty every time. A call that faults leaves nothing half done.
- **Concurrency** is bounded (one call per core by default); a call that waits more than five seconds
  for its turn is turned away. Calls run off the async runtime's threads.
- **Settings** `ENGINE_FUEL`, `ENGINE_MEMORY_MIB`, `ENGINE_CONCURRENCY`, `ENGINE_PATIENCE_SECS`
  (game-api). The sandbox is compiled at start, so a server that cannot run it says so then.

Protocol, in short: `Check` (can the rules be read), `Open` (seats and opening state of a match),
`Picture` (a game's card), `View` (a match as someone sees it), `Play` (a seat's move). The rules and,
for a match, its saved state travel with every request, because nothing is kept between calls.

## How it is kept true

- **The sandbox is the engine, not another engine.** A test runs every catalogue game's `Check`,
  `Open`, `Picture` and `View` in and out of the sandbox and demands identical answers; another plays
  a move and the refusals.
- **Nothing runs rules in-process.** The game crate no longer depends on the engine crate. A clippy
  rule (`disallowed_methods`, in `make lint-panics`) forbids `Session::new`, `resume`, `check_spec`
  and the protocol's `handle` and `answer` anywhere in server code; the one allowance is the unsandboxed
  test engine (`Engine::unsandboxed`, `opening_for_tests`), marked for tests.
- **The server's whole suite runs through the sandbox**: every match played over HTTP in
  `crates/game/tests` goes through it. One test starves the engine of fuel mid-match and checks that the
  player gets the fixed answer, nothing is logged or moved, and the same request then works.

## Costs

- Building the server needs the `wasm32-unknown-unknown` target (the Dockerfile adds it, and
  `make dev` already needed it for the web app). `crates/sandbox/build.rs` builds the module with a
  nested `cargo build`.
- Each call is a little slower than a call in the process (a fresh instance, the request and answer
  copied in and out): about 40 to 70 ms per move over HTTP in a dev build, mostly the engine parsing the rules
  again, as it did before. The compile at start takes a few seconds in release.
- Compiling the module in unoptimised dev builds took minutes, so the workspace builds Cranelift
  and wasmtime with `opt-level = 3` even in dev.
- Rust 1.97 or newer (wasmtime 51).

## Not done

- A **limit on rules at publish** (cells, pieces, text): the engine bounds a board's size and refuses
  steps that go nowhere, and the sandbox bounds the rest; static limits and a probe of random games
  belong to publishing.
- **Quarantining** a game whose calls keep stopping, and a count of stopped calls per game, for the review page.
- A fresh `wasmtime` instance per call uses the default allocator; a pooling allocator would be
  faster if it ever matters.
