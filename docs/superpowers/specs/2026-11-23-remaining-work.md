# What is left after the generality pass

The route in `2026-11-22-rule-blocks-design.md` (slices 1 to 5 and 4c) and the engine's step 5
(`2026-11-25-turn-stages-design.md`, merged as #80) are done. This is the list of what remains, in
the order Claude recommends. Each item that is a design gets its own short design doc when it
starts; nothing here is decided until then. (Dated with the machine's clock, 2026-11-23; the
design docs written during the pass carry later dates from the working sessions.)

## Order from here (Frank, 2026-11-26)

1. **The Flying Knight** (done on `feature/flying-knight`, `2026-11-26-flying-knight-design.md`).
   The look-ahead was cheap: Black's double move is one action of two moves (so check is judged on
   the final position, and a king never steps onto a threatened square); `ATTACKED` gained
   `moves: 2` for White's check and castling; end checks gained `players`. Frank's
   ruling: a king never steps onto a threatened square, not even with the first of its two
   moves (so Black can be stalemated, a draw); the game has no insufficient-material draw (a
   lone king and knight can mate). Not done: a web page and seed entry
   for the catalogue.
2. **Custom link labels** on maps (done on `feature/link-labels`, `2026-11-26-link-labels-design.md`):
   a written link may carry a label, and the full Kalah, with its capture, is in the catalogue.
   Per-player links and sliding along a map's links were left out: no game needs them yet.
3. **Spec versioning** (done on `feature/spec-version`, `2026-11-26-spec-versioning-design.md`):
   every spec states a mandatory `format_version` (1); stored games get it from migration 0006.
4. **Engine step 6, dice and shuffles** (done on `feature/dice-and-shuffles`,
   `2026-11-27-dice-and-shuffles-design.md`): a seed in the match, `ROLL`, ordered piles and
   `SHUFFLE`, every result in the history; the proof is a dice race in the tests. Not done: a
   catalogue entry or book chapter for it.
5. **Engine step 7, hidden information** (`2026-11-28-hidden-information-design.md`, Frank's
   rulings in). **Slice A, the engine and the session, is built** on
   `feature/hidden-information-engine`; **slice B, the server and the web board, is built** on
   `feature/hidden-information-server-web` (stacked on slice A); **slice C, the proof game "Draft"
   and the `BAGGED` score term, is built** on `feature/hidden-information-draft` (stacked on slice
   B), in tests and not in the catalogue.
6. **Engine step 8** (item 5). **8a, sides with their own rules, is built** on
   `feature/sides-with-own-rules` (`2026-11-25-sides-with-own-rules-design.md`): a `PLAYER` test and a
   `start` by player; the proof is Siege, in the tests. **8b, answers out of turn,** is designed
   (`2026-11-25-answers-out-of-turn-design.md`) and built in two slices: **slice 1, `ASK` one
   after another,** is built on `feature/ask-in-order`, the proof Go's dead-stone agreement, in the
   tests; **slice 2, `TOGETHER` (all at the same time, sealed or open; the proof rock-paper-scissors), is built** on `feature/ask-together`.

7. **Catalogue** (Frank, 2026-11-25): the proof games become real ones. Hidden information waits
   for a real game that fits. **Rock Paper Scissors** was in (`feature/catalogue-rock-paper-scissors`); Frank took it out of the catalogue (2026-11-25, "it looks weird"), and it stays as the spec the tests use (`rock_paper_scissors.json`, no page). What it left in the web app stays: the
   island says "Choose one." for a code choice that is not a promotion, and shows what each side
   held once the game is decided, so the loser sees the throw. **Fox and Geese** (sides with their own rules) is in on `feature/catalogue-fox-and-geese`
   (`2026-11-25-fox-and-geese-design.md`; `REPEATS` is now allowed in an end check of a game with
   stages, and the board draws a cross). Next: Snakes and Ladders (dice) and Go's dead-stone
   agreement (its ko rule needs `REPEATS` inside a way, which is still refused in games with
      stages). The "Captured" panel is now left out of games whose skin says `"captured_panel": false`
   (Rock Paper Scissors, Kalah, Hex, Pong Hau Ki).

**Content rulings (Frank, 2026-11-25), done on `feature/content-rulings`:** Janggi's cannon is
the standard one (it moves only by jumping one non-cannon piece and cannot take a cannon); Crazyhouse
follows the standard rules (a pawn double-steps from its own second rank however it got there, and a
dropped rook cannot castle); the shipped four-player game already treats a player who is out as
standard chess.com does (their pieces stay, taking one earns nothing and is no help against a check),
and it is listed in the known limitations as a cut-down game, not the 14x14 one; a piece its owner
moves into its own bag is no longer counted as captured. Go's ko in a game with stages is built
(`2026-11-25-go-standard-design.md`, on `feature/go-ko-in-stages`): the catalogue Go has the
dead-stone agreement and keeps its ko rule.
Found on the way: a cannon's check was read backwards, and a horse attacked through a blocked leg
(mended by attack-set legality, section 4).

**Hex's look (Frank, 2026-11-25):** a rhombus is drawn pointy-topped and leaning like a parallelogram (red top and bottom, blue left and right), with each player's zones as thick borders along the edges instead of tints (`hexBorders` in `board-model.js`); other hex boards (Glinski) are unchanged.

**Snakes and Ladders** is in the catalogue (2026-11-25, `2026-11-25-snakes-ladders-backgammon-design.md`): the first game to use `ROLL`, and it brought `board.art` (a skin draws a map itself). **Backgammon** is designed there and waits for Frank's two rulings.

The other lines of item 4 (building rules, attack-set legality, content questions) and item 6 wait
for Frank's call.

## 1. Verification debt (done, except what is listed)

Checked on `main` after #80:

- **Database tests:** the game, social and game-api suites all pass (game `play` 37/37), run against
  a throwaway local Postgres (Homebrew, port 5466, `TEST_DATABASE_URL`) because Docker's disk is
  still full: its VM has 2 MB free and 105 GB of reclaimable volumes that are not this project's,
  so none were pruned. `modern-chess-db` is up but publishes no port; `make db-up` will not work
  until Docker has space.
- **Browser (the book's live boards, built from the new wasm):** Checkers plays a forced
  capture (only the jump square is offered) and the turn passes after the jump; Kalah, on a
  throwaway page with the test spec, shows counts on its pits, a store that fills, and plays the
  free turn (sowing a3 ends in the store and SOUTH is still to move; sowing a1 then hands the turn
  to NORTH).

Still open:

- A Checkers chain jump through the web app (the book's start position has no chain to reach in a
  few clicks; the engine and session tests cover it, and the opus review compared 300 random games
  with `main`), and the by-hand list from slice 3 (chess check and mate, Racing Kings, Go capture,
  suicide and ko), slice 4's ends and counters, and Kalah's stacked chips for several owners on
  one place.
- Ledgers in `.superpowers/sdd/` hold rulings and deferred minors Frank may not have read; they
  stay until he says so (`2026-11-22-rule-blocks-tests`, `-actions`, `2026-11-23-ends-and-counters`,
  `-hand-effects`, `-lean-state`, `2026-11-24-bags`, `2026-11-25-turn-stages`).

## 2. Deferred minors (done, except what is listed)

Fixed on `fix/deferred-minors`, each with a test that fails without the fix: `PATH` in a stage's
`only` and `LAST` in a repeat's `until` are refused when the spec is read; resuming a state saved
in a stage the rules lack (unknown name, or a stage whose `only` admits nothing) is refused; a
`MOVE` that moves nothing no longer sets `LAST`; a repeat's `until` reads holders (and a game with
no holders makes no holder lookups); `RawLink` refuses unknown keys on a one-way link. The end
checks between the ways of a stage are now documented (SPEC_FORMAT, known limitations).

Ruled by Frank (2026-11-25, "whatever makes more sense"): a piece its owner moves from the board
into a bag is kept, not lost, so it no longer enters the "captured" tally; an enemy piece taken
into the mover's bag still does. Material and `REMOVED` were already right (own bag counted in
material; a piece into a bag is no `REMOVED`). Left as pure tidying, with no behaviour to pin: the
ledgers' stale comments and needless clones (hand-effects, lean-state).

## 3. Unknown fields refused everywhere (done)

Every spec type that reads an object refuses a key it does not know, and the old-form machinery
is gone: `old_forms.rs`, the `old_*` placeholder fields, `GameSpecError::OldForm` and the
`END_TURN` effect. A retired key is an unknown field, with serde's own message
(`2026-11-26-unknown-fields-design.md`). Deferred minors from its review are in the ledger
`2026-11-26-unknown-fields`.

## 4. Rules to design with Frank

- **Per-player links and sliding along a map's links** (custom labels are done): links that
  differ per player, `repeat` on maps. Waiting for a game that needs them.
- **Building rules** (Catan's distance rule, roads that must connect).
- **Attack-set legality** (done on `design/attack-set-legality`, `2026-11-25-attack-set-legality-design.md`):
  a piece attacks the places one of its moves could take on, by the moves' own tests; the second
  geometry is gone. Xiangqi now matches its published perft (44, 1920, 79666), and the horse's leg,
  the elephant's eye and river, the cannon's screen and the soldier's river are read right.
- **A look-ahead test** (done: `ATTACKED` `moves: 2`; deeper than two is refused on purpose, as
  the cost grows with every move looked ahead).
- **Content questions** (ruled 2026-11-25, see item 7): Janggi's cannon, Crazyhouse's double step
  and dropped-rook castling, and the four-player dead pieces are settled. The cannon's check, found
  on the way, is mended by attack-set legality (above).
- **Go's ko in a game with stages** (`2026-11-25-go-standard-design.md`): built. `REPEATS` is
  refused only in a way that goes on to a stage; the catalogue Go agrees the dead stones by group
  and keeps ko. Left out and listed in the known limitations: Japanese scoring, the AGA pass stone,
  handicaps.

## 5. The engine route's later steps (`2026-11-21-engine-generalisation-design.md`)

Each ends with a small original game that needs it.

- **Step 6: dice and shuffles (done).** A random seed lives with the match on the server; rolling
  puts random faces in a bag; shuffling reorders a pile (an `ordered` bag); each result is recorded
  in the history, so a match replays exactly. Proof: a dice race.
- **Step 7: hidden information.** Each place says who can see its contents and whether the count
  shows; the server sends each seat only what it may see; the wasm engine never receives the
  rest. Proof: a card-drafting game. The biggest design of the three; brainstorm first.
- **Step 8: sides with their own rules, and answers out of turn.** `MoveSpec::players`
  extended to pieces, places, actions, stages and end checks; decisions other players take
  outside their turn (everyone discards on a 7). Proof: an asymmetric game.

## 6. Outside the pass

- Recipes and the builder: the tool that hides the long-hand blocks (the route doc left it out on
  purpose).
- Other test games that earn it, into the catalogue (Kalah is in).
- The hub direction: communities form around user-proposed games (memory note
  `project_hub-user-proposed-games`).
- **Per-game builds** (Frank's question, 2026-11-25; to look at later). Could the engine be modular
  enough that a game compiles only the modules it uses, into a smaller or faster binary stored
  per game? Yes, in two senses of very different cost.
  - *Feature-gated blocks (cheap, worth measuring first).* Cargo features for the rule families
    (`dice`, `hidden`, `ask`/`stages`, `bags`/maps, `territory`, n-dimensional boards), so a
    chess-family build leaves the rest out and the spec reader refuses a spec that needs a missing
    feature. The win is mostly the **wasm the browser downloads** (1.49 MB now, unzipped; measured
    below),
    and little at run time, because the hot paths (move generation, attack maps, state clones) do
    not spend their time on the branches of unused features. The reorganization it wants: the
    state of each feature in its own struct (today `Game`'s state carries holders, asked seats,
    focus, seed and tally together), the closed `Effect`/`Test`/`Choice` enums gated by `cfg` or
    replaced by a registry, and a `requires(spec)` function that lists the blocks a spec uses (the
    reader already walks every node). That function is useful on its own: the builder and the book
    can say "this game uses dice and stages".
  - *Compiling the spec to code (expensive, only if something needs the speed).* Generating Rust
    from a spec and building one binary or wasm per game can be much faster (constant geometry,
    no dispatch on tests, move tables), but only an AI or an analysis tool searching millions of
    positions would feel it; a human move costs microseconds today. It brings a build pipeline for
    user-authored games (minutes per submission, a toolchain on the server, a rebuild of every game
    on every engine change and per `format_version`, an artifact store, and one process or
    dynamically loaded module per game instead of one engine). The code would be generated from a
    closed grammar, so there is no sandbox problem; the cost is operational. Running each game as a
    wasm module under a host on the server would give per-game artifacts and isolation together.
  - *Between the two:* a spec is already read into a plan (the blueprints) when a game starts;
    caching and sharpening that plan gets part of the speed with no build step.
  - **Measured (2026-11-26, wasm size only; no move-generation profile yet).** `wasm-pack build
    --release` as `apps/docs/build-engine.sh` runs it: **1,486,902 bytes**, 511 KB gzip, 368 KB
    brotli. The release profile is Cargo's default (no `lto`, `opt-level` 3, 16 codegen units).
    - *Where the bytes are* (`twiggy` on the build with names; the shipped one has them stripped, so
      this is shares of code and data, not exact sizes): about **44% is the spec reader**: the
      `specs` module (serde derives, `validate`, `MapSpec::compile`, `from_spec`) 14%, `serde` 23%
      (mostly the `Content` deserializers that untagged and flattened enums use), `serde_json` 13%.
      The runtime (`logic::play`, `rules`, `game`, `places`, `pieces`) is only about **12%**, the
      session and the bindings 1%, and the rest is `core`/`alloc`/`std` (which includes the
      instantiations of generic code over our types, so it is partly the engine's too).
    - *Feature gating would not pay much.* Function names that belong to one rule family (dice,
      hidden bags, `ASK` and stages, territory, scoring) add up to well under 2% together, and the
      derive code for their enum variants sits inside the shared reader. A chess-only build would
      save a few percent, not a third. What every game needs, the reader, is most of the module, so
      the reorganization of the recommendation below is for clarity (and `requires(spec)`), not for
      size.
    - *The profile is the lever.* Same code, the profile only (all of them through wasm-pack's own
      `wasm-opt`):
      | build | raw | gzip | brotli |
      |---|---|---|---|
      | as shipped | 1,486,902 | 511,508 | 368,112 |
      | `lto = "fat"`, `codegen-units = 1` | 1,385,469 | 467,030 | 345,868 |
      | + `opt-level = "s"` | 1,177,863 | 400,300 | 295,903 |
      | + `opt-level = "z"` | **1,031,642** | **354,923** | **267,040** |
      | + `wasm-opt -Oz` on top | 969,257 | 357,758 | 270,026 |
      `panic = "abort"` changes nothing on wasm. `opt-level = "z"` with `lto` is **31% smaller raw
      and 27% smaller in brotli**, and the extra `-Oz` pass is not worth it.
    - *What it costs at run time* (Node, same wasm API, rough): a Go 19x19 stone placed took 4.6 ms
      with the shipped build and 6.8 ms with `z` (listing the places after it, 0.66 to 1.0 ms);
      Grand Chess was the same within noise (2.1 ms); starting a game was faster with `z`
      (34 to 22 ms, the reader is less inlined). Both are far under what a person notices.
    - *Decision (Frank, 2026-11-26):* leave the build as it is. The numbers are here for when the
      download size or the start-up time becomes a real problem ("if push comes to shove").
    - *Next, if taken:* set `[profile.release]` for the wasm build only (a `profile` in the workspace
      would also slow the native server; wasm-pack takes `CARGO_PROFILE_RELEASE_*` or a dedicated
      profile via `--profile`), and keep the opt-level 3 build for native. A bigger cut would come
      from the reader: the `Content` deserializers come from `untagged` and `flatten`, and a
      hand-written `Deserialize` (or `tag`ged enums where the format allows) for the hot spec types
      would shrink them, at the cost of format work. Not measured.
  - **Recommendation:** measure before deciding (a size breakdown of the wasm, and a profile of
    move generation on the largest games: 19x19 Go, Grand Chess). Take the reorganization that
    helps either way: separate the spec reader from the runtime, split the state by feature, add
    `requires(spec)`, and keep the session's interface narrow so the engine behind it can be swapped.
