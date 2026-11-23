# What is left after the generality pass

The route in `2026-11-22-rule-blocks-design.md` (slices 1 to 5 and 4c) and the engine's step 5
(`2026-11-25-turn-stages-design.md`, merged as #80) are done. This is the list of what remains, in
the order Claude recommends. Each item that is a design gets its own short design doc when it
starts; nothing here is decided until then. (Dated with the machine's clock, 2026-11-23; the
design docs written during the pass carry later dates from the working sessions.)

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

## 2. Deferred minors (small fixes, one slice)

From the #80 review: `PATH` in a stage's `only` is accepted but can never be formed; `LAST` in a
move's repeat `until` is accepted but names no place; restoring a saved state does not check its
stage against the spec (unknown name, or a stage whose `only` admits nothing); `LAST` is set by a
`MOVE` that moves nothing; end checks are not judged between the ways of a stage (as with the old
jump lock; undocumented). From #78: a move blueprint's `until` does not read holders; a piece
moved from the board into a holder does not count in the session's captured tally, material or
`REMOVED`; `RawLink` ignores unknown keys; hot-path empty-map lookups.

## 3. Unknown fields refused everywhere (small design)

A misspelled key is silently ignored in places (a top-level key is the worst). Make every spec
type refuse unknown fields, which also retires the placeholder fields that refuse old keys
(`old_*: OldField`). Effects should be as strict as the test blocks (route doc, "Carried into
later slices").

## 4. Rules to design with Frank

- **Custom link labels and per-player links** on maps: Kalah's capture of the opposite pit,
  sliding along a map's links, links that differ per player. Unlocks the full Kalah, which could
  then enter the catalogue.
- **Building rules** (Catan's distance rule, roads that must connect).
- **Attack-set legality.** A piece's attack set ignores its moves' conditions and `until`, so a
  slide threatens fewer squares than it reaches, and a zone test on a move does not limit its
  threats (Xiangqi's flying general relies on that today). Reworking it changes which moves are
  legal; waits for Frank's rulings.
- **A look-ahead test**, for the Flying Knight (a double move whose check rule looks two moves
  ahead). Worth building only if Frank wants the game; documented in the known limitations.
- **Content questions:** `janggi_cannon.json` lets a cannon capture an enemy cannon (standard
  Janggi forbids it) and has a never-failing test; Crazyhouse double step and dropped-rook
  castling; REMOVED ignoring out-of-game players' pieces in a 4-player game.

## 5. The engine route's later steps (`2026-11-21-engine-generalisation-design.md`)

Each ends with a small original game that needs it.

- **Step 6: dice and shuffles.** A random seed lives with the match on the server; rolling puts
  random faces in a place; shuffling reorders a pile; each result is recorded in the history, so a
  match replays exactly. Proof: a dice race. Needs ordered piles (decks), which 4c-2 left out.
- **Step 7: hidden information.** Each place says who can see its contents and whether the count
  shows; the server sends each seat only what it may see; the wasm engine never receives the
  rest. Proof: a card-drafting game. The biggest design of the three; brainstorm first.
- **Step 8: sides with their own rules, and answers out of turn.** `MoveSpec::players`
  extended to pieces, places, actions, stages and end checks; decisions other players take
  outside their turn (everyone discards on a 7). Proof: an asymmetric game.

## 6. Outside the pass

- Recipes and the builder: the tool that hides the long-hand blocks (the route doc left it out on
  purpose).
- Kalah, and other test games that earn it, into the catalogue.
- The hub direction: communities form around user-proposed games (memory note
  `project_hub-user-proposed-games`).
