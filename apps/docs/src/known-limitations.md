# Known limitations

Writing this guide meant trying every example against the engine. This page lists what
turned up. It describes how things behave **as of November 2026**. Some of these are rough
edges that will probably be fixed, so check them against the current engine before relying on
them.

## Sharp edges when writing a spec

- **Positions off the board** in `starting_positions` are ignored without warning. The piece
  just does not appear.
- **A `CODE` choice always asks.** There is no "skip". For optional promotion, list the piece's
  own code in `options`. A promotion with one option is not a choice: write a plain `SET_CODE`.
- **A piece in the hand is chosen, not placed.** `HAND` picks a code from the hand, and `START` then
  names that piece: a test can look at its `codes`, and an offset from `START` cannot be formed.
- **Tests do not affect threats.** A move's tests and its `repeat.until` decide which
  moves are offered, not what a piece threatens. (The one exception is a test at the start of the
  move that asks for an empty `TARGET`: such a move threatens nothing.) A piece kept in a zone by `IN_ZONE` tests still
  threatens squares outside it. Xiangqi's flying general relies on this.
- **`COUNT` asks for at least one piece unless told otherwise.** Its `min` is `1` when left out,
  so `"max": 0` on its own is refused (`min` would be above `max`). Write `"min": 0, "max": 0`,
  or a `NOT` around the `COUNT`, for "no piece here".
- **A region that cannot be formed is false.** A place off the board, or a `PATH` that crosses a
  hole or the edge, makes `COUNT` and `ATTACKED` over it false. (A `PATH` that is not a straight
  line is refused when the spec is read.) `NOT`
  flips that, so "nothing stands on the square at `[0, 1]`" holds when that square is off the
  board.
- **A `LINE` does not stop at holes.** It skips disabled squares and runs on to the edge of the
  board. A `PATH`, by contrast, cannot cross a hole.
- **`ATTACKED` knows one attacker, the enemy.** A test cannot ask whether a square is defended by
  the mover's own pieces.
- **Offsets, `PATH` and `LINE` work on grids only.** A map refuses them; use `AROUND` there.
- **`no_opponent_check`, `mandatory_capture` and `jump_enabled` apply to the whole game.** You
  cannot set them for one piece or one move.
- **`AREA_SCORE` counts the board as it is.** The players cannot agree that a stone is dead, so
  it has to be captured before the passes. The score shown during play is the count as if the
  game ended at that moment, so early in a game it swings wildly.
- **`liberty_capture` connects pieces along rows and columns only**, and a placing action
  cannot be capped (no "you have 12 stones left").
- **A dropped pawn may double-step from any rank.** A dropped piece has never moved, and the
  pawn's double step asks only that, and that the square ahead is empty. In standard Crazyhouse
  only a pawn on its second rank may double-step, so `crazyhouse.json` differs there.

## The command-line player

The CLI (`modern-chess-cli`) is a quick way to try a spec, not a finished product.

- It announces every win as `Checkmate! <player> wins!` and every draw as
  `Stalemate! It's a draw.`, whatever the real reason.
- It does not say which of the places on offer capture: every place of a `MOVE` choice reads
  `(Move)`.
