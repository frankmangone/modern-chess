# What is left after the generality pass

The route in `2026-11-22-rule-blocks-design.md` (slices 1 to 5 and 4c) and the engine's step 5
(`2026-11-25-turn-stages-design.md`, merged as #80) are done. This is the list of what remains, in
the order Claude recommends. Each item that is a design gets its own short design doc when it
starts; nothing here is decided until then. (Dated with the machine's clock, 2026-11-23; the
design docs written during the pass carry later dates from the working sessions.)

## 1. Verification debt (first, small)

- The game-service database tests have not run since before #78: Docker's disk is full on this
  machine, so Postgres has no published port. #78 (bags) and #80 (stages) are untested there.
  Needs space freed or the container recreated (`make db-down` wipes local data: ask Frank).
- Browser checks never done: slice 4 (ends and counters), Kalah's board (stacked chips for
  several owners on one place), the Checkers jump through the web app after stages, and the
  by-hand list from slice 3 (chess check and mate, Racing Kings, Checkers forced jump, Go
  capture, suicide and ko).
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
