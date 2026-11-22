# Known limitations

Writing this guide meant trying every example against the engine. This page lists what
turned up. It describes how things behave **as of November 2026**. Some of these are rough
edges that will probably be fixed, so check them against the current engine before relying on
them.

## Sharp edges when writing a spec

- **Positions off the board** in `starting_positions` are ignored without warning. The piece
  just does not appear.
- **Promotion is always a choice the player must make.** There is no "skip". For optional
  promotion, list the piece's own code in `options`.
- **Tests do not affect threats.** A move's `conditions` and its `repeat.until` decide which
  moves are offered, not what a piece threatens. A piece kept in a zone by `IN_ZONE` tests still
  threatens squares outside it. Xiangqi's flying general relies on this.
- **`COUNT` asks for at least one piece unless told otherwise.** Its `min` is `1` when left out,
  so `"max": 0` on its own is refused (`min` would be above `max`). Write `"min": 0, "max": 0`,
  or a `NOT` around the `COUNT`, for "no piece here".
- **A region that cannot be formed is false.** A place off the board, or a `PATH` that is not a
  straight line or crosses a hole or the edge, makes `COUNT` and `ATTACKED` over it false. `NOT`
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
- **`liberty_capture` connects pieces along rows and columns only**, and an unlimited supply
  cannot be capped (no "you have 12 stones left").

## The command-line player

The CLI (`modern-chess-cli`) is a quick way to try a spec, not a finished product.

- It cannot **drop** pieces from the hand, so Shogi and Crazyhouse can be played only on the
  board.
- It announces every win as `Checkmate! <player> wins!` and every draw as
  `Stalemate! It's a draw.`, whatever the real reason.
- If its input ends (for example when you pipe moves into it), it loops forever instead of
  exiting, so cap the output when you script it.
