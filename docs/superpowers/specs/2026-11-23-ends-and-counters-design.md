# Ends and counters (slice 4 of the generality pass)

Slice 4 of `2026-11-22-rule-blocks-design.md`. Slices 1–3 made tests, a turn's steps and the rules
on every move into blocks. This slice does the same for how a game ends: win conditions, draw
conditions, what happens to a player with no way to play, and the counters those read.

Frank asked for slices 4, 5 and step 5 to be built overnight (2026-11-22), without questions. The
choices below are Claude's calls, recorded as such; each is cheap to undo while nothing else
depends on it.

## Why

Five special forms still decide how a game ends: `win_conditions` (five fixed types), the
`draw_conditions` record (four fixed rules), `leader` (read only by the no-legal-move outcome and
Three-check's counter), `stalemate_loses`, and counters that live in the engine (`check_counts`,
the fifty-move scan of the history). A person or an agent writing a new game cannot say "the first
to capture five stones wins" or "a draw after sixty quiet moves" without the engine learning a new
name for it.

## Goals

- `win_conditions`, `draw_conditions`, `leader` and `stalemate_loses` leave the format. Every spec
  that used them is rewritten and plays exactly as before, except the two named changes below.
- Every block added is used by more than one idea.
- Grid games stay as fast as they are (timing run within 5% of the slice 3 branch on chess,
  Shogi and Go; identical move counts).

## Non-goals

- Interned state and the attack-set rework (slice 5); turn stages (step 5).
- Hands and supplies as places (step 4c).

## Anti-goals

- A silent end rule: an unknown test, counter, outcome or player in an end check is refused when
  the spec is read.
- A block that only one game could use. *(One exception, ruled below: `TERRITORY`.)*

## The blocks

### `end_checks`

A top-level list, judged in order after every turn, once the turn has passed to the next player.
The first check whose test holds ends the game or removes a player, and no later check runs.

```json
"end_checks": [
  { "when": { "test": "COUNTER", "name": "CHECKS", "min": 3 }, "then": { "outcome": "WIN" } },
  { "as": "NEXT", "when": { "test": "STUCK" }, "then": { "outcome": "DRAW" } }
]
```

| Field | Meaning |
|---|---|
| `as` | Whose point of view the test is judged from, as `ALLY` and `ENEMY` are seen from "the mover" in a way: `MOVER` (default), the player who just played; `NEXT`, the player about to play. Zones are read with that player's cells. |
| `when` | A test. It names no place (`START`, `TARGET`, `EACH` outside a `SOME`) and no `REMOVED` or `MOVED`: there is no way being worked out. |
| `then.outcome` | `WIN`: the `as` player wins. `LOSE`: the `as` player is out of the game (the turn order shrinks; the game ends when one player is left, who wins). `DRAW`: the game ends with no winner. `HIGHEST`: the player with the highest `score` wins; a tie is a draw. |

### New tests

| Test | Written | Meaning |
|---|---|---|
| `STUCK` | `{ "test": "STUCK" }` | The player to move has no way to play. Allowed only in an end check with `as: NEXT`; refused anywhere else when the spec is read. |
| `PASSES` | `{ "test": "PASSES", "in_a_row": 2 }` | The last `in_a_row` turns, by anyone, changed nothing. |
| `COUNTER` | `{ "test": "COUNTER", "name": N, "min": a, "max": b }` | The `as` player's counter is within `[min, max]` (`min` defaults to 0, `max` to no limit). Inside a way it reads the counter as the turn began. |
| `SOME` | `{ "test": "SOME", "in": R, "that": T }` | At least one place of region `R` passes `T`, judged with `EACH` at that place (an inner `SOME` replaces the outer `EACH`; each region is taken before it runs). |
| `MOVED` | `{ "test": "MOVED", "where": F }` | The way has so far moved, along the board, a piece that matches `F`, as it stood before. Where `REMOVED` looks at what left, this looks at what travelled. Refused in an end check. |
| `REPEATS` | `{ "test": "REPEATS", "times": 3 }` | The position is on record at least `times` times (default 1, which is the ko rule of slice 3). In an end check the position just reached is already on record, so threefold repetition is `times: 3`. |
| filter `except` | `{ "except": ["KING"] }` | The piece's code is none of these. Combinable with the other filter fields except `empty`. |
| `ATTACKED` `keeping` | `"keeping": { "codes": ["KING"] }` | A capture counts as a threat only if the attacker still has a piece that matches the filter after it, when it had one before. This is Atomic's "two kings may stand side by side", written where it applies instead of read from `leader`. It is only worked out in games where a move removes more than its target (as today). |

### Counters and scores

`counters: ["CHECKS", "QUIET"]` declares the names. Each player holds a number for each, starting
at 0, saved with the game. Two effects change them, as steps (usually in `every_action`):

| Effect | Written | Meaning |
|---|---|---|
| `ADD` | `{ "effect": "ADD", "counter": N, "by": 1, "to": "MOVER" }` | Adds `by` (a whole number, default 1) to the mover's counter, or to every player's with `"to": "ALL"`. |
| `RESET` | `{ "effect": "RESET", "counter": N, "to": "ALL" }` | Sets it to 0. |

`score` is the game's one score, a list of terms summed for each player. It is read by the
`HIGHEST` outcome and shown live by the session:

| Term | Written | Meaning |
|---|---|---|
| `PIECES` | `{ "term": "PIECES", "where": F }` | The player's pieces on the board that match `F` (`F` has no `owner`). |
| `TERRITORY` | `{ "term": "TERRITORY", "where": F }` | The empty places in each orthogonally joined empty region whose neighbours, among the pieces that match `F`, belong to the player alone. |
| `COUNTER` | `{ "term": "COUNTER", "name": N }` | The player's counter. |
| `POINTS` | `{ "term": "POINTS", "value": 7.5, "to": ["WHITE"] }` | A fixed number, for the named players (komi). |

## What each old form becomes

- **`PIECE_IN_ZONE`**: `SOME in BOARD where ALLY codes[K] that IN_ZONE at EACH` as `MOVER` → `WIN`.
- **`OPPONENT_BARE`**: `COUNT in BOARD where ENEMY except[exempt] max 0` → `WIN`.
- **`CONNECTION`** (Hex, the Catan sampler): `SOME in BOARD where ALLY codes[…] that ALL[ SOME in GROUP of EACH … that IN_ZONE … ]`, one inner `SOME` per zone → `WIN`. With `at_least` fewer than all zones, an `ANY` of the combinations.
- **`CHECK_COUNT`** (Three-check): `every_action` gets `if ATTACKED in BOARD where ENEMY KING by MOVER then ADD CHECKS`; an end check reads `COUNTER CHECKS min 3` → `WIN`.
- **`AREA_SCORE`** (Go): `PASSES in_a_row 2` → `HIGHEST`, with `score: [PIECES STONE, TERRITORY STONE, POINTS 7.5 to WHITE]`.
- **Checkmate and stalemate**: `as NEXT`, `ALL[STUCK, ATTACKED in BOARD where ALLY KING by ENEMY]` → `LOSE`, then `as NEXT`, `STUCK` → `DRAW`. Games where stalemate loses write only `STUCK` → `LOSE`.
- **Repetition**: `REPEATS times n` → `DRAW`.
- **Fifty moves**: `counters: ["QUIET"]`; `every_action`: `if ANY[REMOVED ENEMY, MOVED PAWN] then RESET QUIET to ALL else ADD QUIET to ALL`; end check `COUNTER QUIET min 100` → `DRAW`.
- **Insufficient material**: per side, `ALL[COUNT KING =1, COUNT except[KING,BISHOP,KNIGHT] max 0, COUNT [BISHOP,KNIGHT] max 1]`, for `ALLY` and `ENEMY` → `DRAW`.

## Behaviour changes, named

- **Atomic ends when a king blows up.** Today the game goes on if the player whose king was
  blown up still has other pieces (found while designing this slice; slice 3 introduced it, the
  old `leader` check ended it). An end check `as NEXT`, `COUNT in BOARD where ALLY KING max 0` →
  `LOSE` restores it.
- **Pong hau k'i's threefold draw comes after 12 plies, not 18.** The position record counted each
  piece's has-moved flag, which no rule of that game reads. The flag is now part of the record
  only in games where some test, filter or flag reads `moved` (castling, double steps).
- A game saved before this slice restores with its counters at 0, so a fifty-move count starts
  again from the restore.

## Checked when a spec is read

An unknown `outcome` or `as`; `STUCK` anywhere but an end check with `as: NEXT`; a place name or
`REMOVED` or `MOVED` in an end check; a `COUNTER` test, effect or term naming an undeclared counter;
`HIGHEST` with no `score`; a `score` term with an `owner` in its filter; `ADD` with `by` 0;
`ATTACKED keeping` with `owner` or `empty`; `PASSES` with `in_a_row` 0; `REPEATS` with `times` 0.
The old keys `win_conditions`, `draw_conditions`, `leader` and `stalemate_loses` are refused with
`OldForm`.

## The engine

- `check_game_over` records the position, then judges `end_checks` in order, working out the ways
  once and only when a check reads them (`STUCK`); the ways are then reused to start the turn.
- `REPEATS` in an end check judges the position as it stands (no turn handed on, no flags ticked).
- `Way` gains `counts` (the counter changes the way makes, applied when it is played) and `moved`.
- `leader`, `stalemate_loses`, `players_with_configured_leader`, `leader_in_check*`,
  `WinConditionKind`, `connection.rs`, `check_area_score`, `check_counts` and `Rules`'s draw fields
  go. `scoring.rs` keeps the territory count behind `score`.

## Rulings

- **`TERRITORY` is a block only Go uses.** It cannot be built from regions without an "each
  region of" iteration, which no other game asks for yet. It is kept as a score term, not a test,
  so the rest of the vocabulary stays clean; it is the one block against the anti-goal.
- **Counters are per player**, with `to: ALL` for shared ones, so one mechanism serves Three-check
  (per player) and the quiet-move count (shared).
- **`as` is one field on the check, not a mode of each test**, so every existing test works from
  either point of view without changes.

## Proving it

- Every catalogue spec, book board and test spec is rewritten; `catalogue_counts` stays identical.
- New tests for each block and refusal above, Atomic ending on a king blown up with pieces left,
  Pong hau k'i's 12-ply draw, Three-check, a fifty-move draw, threefold repetition, Hex's win,
  Go's score through `HIGHEST`, checkmate and stalemate in 2 and 4 players.
- The timing run stays within 5% on chess, Shogi and Go.

## Left to the builder

Type names and layout; error texts; how `SOME` and `GROUP` avoid copying; the order of commits.
