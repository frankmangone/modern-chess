# 11. Designing your own game

You now know every part of the spec. This chapter is about *using* it: a way of working that
keeps you out of trouble, a lookup table for "how do I express X?", and a short guide to what
goes wrong.

## A way of working

**1. Start from the closest game that exists.** Copy a file from `crates/engine/specs/` and
change it. You almost never need to start from an empty page.

| If your game has... | Start from |
|---------------------|------------|
| chess-like pieces and a king | `chess.json` |
| a different board size or extra pieces | `capablanca.json`, `grand_chess.json`, `gothic.json` |
| only steps and slides, no special rules | `chaturanga.json` |
| captured pieces coming back | `shogi.json` or `crazyhouse.json` |
| a non-chess aim (reach a square, count checks) | `king_of_the_hill.json`, `three_check.json` |
| jumping and forced captures | `checkers.json` |
| one side with a different turn (a double move) | `flying_knight.json` |
| pieces that clone or convert | `ataxx.json` |
| placing pieces on an empty board | `go.json` |
| more than two players | `4player_chess.json` |
| a board with holes in it | `disabled_positions` (see the reference) |
| a hex board | `"shape": "hex"` (see the reference), `hex.json`, `glinski.json` |

**2. Build in layers, and test after each one.**

1. `format_version` (always `1`), the name, board, players, turn order, the `MOVE` action, and
   **one** piece with **one** move. Run it. A spec without `format_version` is refused.
2. The rest of the pieces. Run it.
3. The special moves (promotion, castling, anything with tests and effects). Run it.
4. The rules on every move (`every_action`, `must`), then the endings: `end_checks` for wins and
   draws, with the counters and the `score` they read. Run it.

When something breaks you will know which layer did it.

**3. Test on a tiny board.** Every chapter used a scratch copy with two or three pieces on it.
Do the same. It is the quickest way to check one rule, and you can set up the situation you want
(a pawn one step from promotion, a king next to the hill) without playing thirty moves to reach
it. Delete pieces from `starting_positions` and shrink `dimensions`.

**4. Put the opposing players side by side.** Each player's `starting_positions` should look
like a mirror of the other's. If a move depends on direction, check it for *both* players.

## How do I express...?

| I want... | Use | Chapter |
|-----------|-----|---------|
| a turn that picks a piece and moves it | the action `MOVE`: a `PIECE` choice, then a `MOVE` choice | 1 |
| a piece that steps | `step`, a test that `TARGET` is empty, and a `MOVE` effect in `do` | 1 |
| a piece that captures | a test that `TARGET` holds no ally, then the `MOVE` effect | 2 |
| a sliding piece | `repeat: { "loop": true }` | 2 |
| a slide with a limit | `repeat: { "times": N }` | 2 |
| a piece that must be protected | a check entry in `every_action`, and a `STUCK` end check for checkmate | 2, 5 |
| a rule that applies to every move | an `every_action` entry, with `for` to name the actions | 2, 10 |
| a move that may not give check | `NOT` `ATTACKED` of the enemy king `by` `MOVER`, in `every_action` | 5 |
| moves that depend on direction | `direction` on each player | 3 |
| a named area of the board | a zone under `zones`, and `IN_ZONE` | 3 |
| promotion | an `if` over a zone with a `SET_CODE` effect | 3 |
| promotion with a choice | a `CODE` choice before the `SET_CODE` | 3 |
| optional promotion | include the piece's own code in the `CODE` choice's `options` | 7 |
| "only once, from the start" | `COUNT` at `START` with `"moved": false` | 4 |
| a move that needs an empty square | `NOT` of a `COUNT` at that square | 4 |
| a move that needs a clear path | `COUNT` in a `PATH` with `"min": 0, "max": 0` | 4 |
| a move that needs friends nearby | `COUNT` in an `AROUND` with `"owner": "ALLY"` | 4 |
| a move only some players may make | `players` on the move | 4 |
| a move that remembers something | a `SET_FLAG` effect, and a `COUNT` whose filter has a `flag` | 4 |
| a move that changes another piece | a `MOVE`, `REMOVE`, `SET_OWNER`, `SET_CODE` or `PLACE` effect | 4, 8 |
| a move that changes every neighbour | a `FOR_EACH` effect over an `AROUND` region | 8 |
| keeping a piece in a region | an `IN_ZONE` test on each of its moves | 6 |
| a piece that can be blocked | `NOT` of a `COUNT` at the blocking square, or a `COUNT` in a `PATH` | 6 |
| a slide that stops somewhere else | a test in `repeat.until` | 6 |
| a threat a piece cannot act on | `threat_only` | 6 |
| a different way to win | an entry of `end_checks` with a test and the outcome `WIN` | 5 |
| a different kind of draw | an entry of `end_checks` with the outcome `DRAW` (`REPEATS`, a counter, a count) | 5 |
| a player with no move loses (or draws) | an `end_checks` entry `as` `NEXT` with `STUCK` | 5 |
| "the first to N" | a counter, an `ADD` effect in `every_action`, and a `COUNTER` test | 5 |
| a player is out when a piece is gone | an `end_checks` entry `as` `NEXT` that counts their pieces, outcome `LOSE` | 5, 8 |
| the best score wins | `score`, and an end check with the outcome `HIGHEST` | 5, 10 |
| captured pieces coming back | a `HAND` bag, `removed_to`, `captured_as`, a `DROP` action with a `HELD` choice | 7 |
| a store of pieces off the board (a hand, a bank, a supply) | a bag in `bags`, with `capacity` and `start` if needed | 7, 13 |
| a place that holds many pieces | a map place with `capacity` and `start` | 13 |
| moving, adding or taking several pieces at once | `MOVE`, `PLACE` or `REMOVE` with a bag and a `count` | 13 |
| a walk along a ring, a sowing | a `FOR_EACH` over a `FOLLOW`, on one-way links | 13 |
| a score of what the places hold | the `KEPT` score term | 13 |
| a rule about a whole row or column | a `COUNT` in a `LINE` | 7 |
| forced captures, chain jumps | `"must": [ REMOVED where owner ENEMY ]`, a `STAGE` effect | 9 |
| "if you can do this, you must" | a test in `must` | 9 |
| the same player moving again (a chain of jumps, a free turn) | `turns.stages` and a `STAGE` effect | 9, 13 |
| a move that ends the turn where others go on (a crowning jump) | the `STAGE` effect in the `else` of an `if`, so the way that ends leaves it out | 9 |
| a turn that makes a piece, with no bag to take it from | an action with a `PLACE` choice and a `PLACE` effect | 10 |
| passing | an action with an empty `do`: `{ "name": "PASS", "do": [] }` | 10 |
| capture by surrounding (liberties) | a `FOR_EACH` and a `COUNT` of `empty` places `AROUND` a `GROUP`, in `every_action` | 10 |
| no move may repeat a position (ko) | `NOT` `REPEATS`, in `every_action` | 10 |

## When something goes wrong

### The engine tells you

A mistake in the *structure* of the file stops the game from loading, with a message that names
the problem:

| Message | What it means |
|---------|---------------|
| `UnknownPieceInStartingPosition("MARCHR")` | a piece code in `starting_positions` is not in `pieces`. A typo, usually. |
| `UnknownPlayerInTurnOrder("BLAK")` | a name in `turns.order` is not a player. |
| `InvalidDirectionMatrix([[1, 1], [1, 1]])` | `direction` is not a rotation or reflection. Use one of the four from chapter 1. |
| `DuplicatePlayerName(...)` | two players share a name. |
| `PositionDisabled(...)` | you placed a piece on a square listed in `disabled_positions`. |
| `JsonError(... missing field `board` ...)` | a required field is missing (a spec with no `actions`, or a move with no `do`, too), or the JSON is malformed. |
| `JsonError(... unknown variant `CONT` ...)` | a misspelled test, region, choice or effect. The message lists the ones that exist. |
| `JsonError(... a step carries exactly one of ...)` | a step that has none of `choose`, `test`, `effect` and `if`, or more than one. |
| `JsonError(... unknown field `owner` ...)` | a field in the wrong block, such as `owner` written beside `in` instead of inside `where`; a misspelled key; or a key the format has retired, such as `conditions` or `jump_enabled` (the spec format lists what replaced each). |
| `InvalidTest(...)` | a test names something the game does not have: a zone not under `zones`, a piece code no piece has, or a `COUNT` whose `min` is above its `max`. Or a `PATH` has no line fixed when the spec is read: its ends are the same place, its line is not straight, or it has an offset between `START` and `TARGET` on a sliding move. Or `{ "empty": true }` is written beside another filter field, or a `REPEATS` is in a game with `turns.stages`, anywhere but an end check. The message says which test and where. |
| `InvalidStep(...)` | a step of an action or a move is in the wrong place: it names `TARGET` before any `MOVE` or `PLACE` choice, `START` before a `PIECE` or `HELD` choice, or `EACH` outside a `FOR_EACH`; a `MOVE` choice with no `PIECE` before it; a `{ "chosen": ... }` that no `CODE` choice names; a choice inside a `FOR_EACH`; a test written bare in a `FOR_EACH`; a choice other than `CODE` in a piece's move; a `PATH` in an action; a repeated action name; an empty `actions` list; or an `every_action` entry with a choice, an empty `for`, or a `for` naming an action the game does not have. The message names the step, as `step 2 of action DROP`. |
| `UnknownPieceInRule(...)` | a piece code in a `PLACE` or `SET_CODE` effect, or in a `CODE` choice's options, that no piece has. A typo, usually. |
| `DuplicateZone("PALACE")` | two zones share a name. |
| `UnknownPlayerInZone { ... }` | a zone's `places` lists a name that is not a player. A typo, usually. |

### The engine stays quiet

Some mistakes do **not** stop the game. They just make it behave oddly. These are the ones to
look for when "nothing happens":

- **A starting square off the board.** The piece does not appear. Check the addresses against
  `dimensions`, remembering that counting starts at **zero**.
- **A `COUNT` that asks for at least one.** `min` is `1` when you leave it out. A guard meant to
  say "this square is empty" needs a `NOT` around the `COUNT`, or `"min": 0, "max": 0`. Forget
  it and the move is offered only when the square is *occupied*.
- **A test that is too strict.** A move is offered only when every test passes. A piece that can
  never capture usually asks for an *empty* target. A piece that should capture must ask only that
  `TARGET` holds no ally.
- **An effect on the wrong place.** `SET_FLAG` at `START` runs after a `MOVE` has emptied it, and
  does nothing. A step sees the board as the steps before it left it, so write the effect at the
  place where the piece is *then*, usually `TARGET`.
- **A move written from the wrong side.** Steps are written as if the player moves *up*. If
  BLACK's pieces move in the wrong direction, check BLACK's `direction`.
- **A zone written in the wrong coordinates.** Zone squares are absolute board addresses and are
  not rotated, unlike `step`.

For the rest, see [Known limitations](known-limitations.md).

## Where to go next

- The [spec reference](reference.md) is a complete list of every field and test.
- The tests in `crates/engine/src/tests/` are a catalogue of worked examples. If you wonder how
  a rule behaves, there is often a test for it.
- Start small: a variant of a game you already know is the fastest way to learn what the engine
  can do. Then invent.
