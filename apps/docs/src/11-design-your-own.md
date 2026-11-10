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
| pieces that clone or convert | `ataxx.json` |
| more than two players | `4player_chess.json` |
| a board with holes in it | `disabled_positions` (see the reference) |

**2. Build in layers, and test after each one.**

1. Board, players, turn order, and **one** piece with **one** move. Run it.
2. The rest of the pieces. Run it.
3. The special moves (promotion, castling, anything with conditions). Run it.
4. The endings: win conditions, draws, `leader`. Run it.

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
| a piece that steps | `step` plus an `actions` entry | 1 |
| a piece that captures | an `ENEMY` → `CAPTURE` action | 2 |
| a sliding piece | `repeat: { "loop": true }` | 2 |
| a slide with a limit | `repeat: { "times": N }` | 2 |
| a piece that must be protected | `leader` | 2 |
| moves that depend on direction | `direction` on each player | 3 |
| a named area of the board | a `POSITION` condition (a zone) | 3 |
| promotion | `modifiers` with `TRANSFORM` | 3 |
| optional promotion | include the piece's own code in `options` | 7 |
| "only once, from the start" | `PIECE_FIRST_MOVE` at `[0, 0]` | 4 |
| a move that needs another move | `DEPENDS_ON` | 4 |
| a move that needs a clear path | `PATH_PIECE_COUNT` with `max: 0` | 4 |
| a move only some players may make | `players` on the move | 4 |
| a move that remembers something | `SET_STATE` and `CHECK_STATE` | 4 |
| a move that changes another piece | a `MOVE`, `CAPTURE`, `CONVERT` or `EXPLODE` side effect | 4, 8 |
| keeping a piece in a region | `restrictions` with a zone | 6 |
| a piece that can be blocked | `LEG_EMPTY`, or `PATH_PIECE_COUNT` | 6 |
| a threat a piece cannot act on | `threat_only` | 6 |
| a different way to win | `win_conditions` | 5 |
| a different kind of draw | `draw_conditions`, `stalemate_loses` | 5 |
| captured pieces coming back | `hand_enabled`, `demotes_to`, `drop_restrictions` | 7 |
| forced captures, chain jumps | `mandatory_capture`, `jump_enabled` | 9 |

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
| `JsonError(... missing field `board` ...)` | a required field is missing, or the JSON is malformed. |

### The engine stays quiet

Some mistakes do **not** stop the game. They just make it behave oddly. These are the ones to
look for when "nothing happens":

- **A starting square off the board.** The piece does not appear. Check the addresses against
  `dimensions`, remembering that counting starts at **zero**.
- **A misspelled condition.** A condition code the engine does not recognise is treated as
  *false*, so the move is never offered. A piece that has no moves at all is often this. Check
  every `condition` name against the reference, and every zone name against your `conditions`
  list.
- **A move with no matching action.** If a move's `actions` has no entry for the state of the
  landing square, nothing happens there. If a piece cannot capture, it needs an `ENEMY` →
  `CAPTURE` action.
- **A move written from the wrong side.** Steps are written as if the player moves *up*. If
  BLACK's pieces move in the wrong direction, check BLACK's `direction`.
- **A zone written in the wrong coordinates.** Zone squares are absolute board addresses and are
  not rotated, unlike `step`.

For the rest, see [Known limitations](known-limitations.md).

## Where to go next

- The [spec reference](reference.md) is a complete list of every field and condition.
- The tests in `crates/engine/src/tests/` are a catalogue of worked examples. If you wonder how
  a rule behaves, there is often a test for it.
- Start small: a variant of a game you already know is the fastest way to learn what the engine
  can do. Then invent.
