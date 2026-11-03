# Known limitations

Writing this guide meant trying every example against the engine. This page lists what
turned up. It describes how things behave **as of November 2026**. Some of these are rough
edges that will probably be fixed, so check them against the current engine before relying on
them.

## Destroying your own leader is allowed

The engine never offers a move that leaves your leader *attacked*, but it does offer a move that
destroys your own leader outright, such as an Atomic Chess capture whose explosion takes your own
king with it. Such a move is legal and **loses the game** for the player who made it. Real Atomic
Chess forbids the move instead, so decide which you want before designing a similar game.

## Sharp edges when writing a spec

- **Positions off the board** in `starting_positions` are ignored without warning. The piece
  just does not appear.
- **Unknown condition names** make the move unavailable, without an error. This is deliberate
  (a mistake should block a move, never allow one), but a typo is silent. (In `repeat.until` an unknown name means "never stop", as chapter 6 uses
  on purpose.)
- **Promotion is always a choice the player must make.** There is no "skip". For optional
  promotion, list the piece's own code in `options`.
- **A zone only looks at where a move lands.** It cannot ask "did this move *leave* the zone?".
  Real Shogi lets a piece promote on leaving the promotion zone; the spec cannot say that.
- **Zone restrictions do not affect threats.** A piece restricted to a zone still threatens
  squares outside it. Xiangqi's flying general relies on this.
- **`ALLY_ON_FILE` works only for drops.**
- **`no_opponent_check`, `mandatory_capture` and `jump_enabled` apply to the whole game.** You
  cannot set them for one piece or one move.

## The command-line player

The CLI (`modern-chess-cli`) is a quick way to try a spec, not a finished product.

- It cannot **drop** pieces from the hand, so Shogi and Crazyhouse can be played only on the
  board.
- It announces every win as `Checkmate! <player> wins!` and every draw as
  `Stalemate! It's a draw.`, whatever the real reason.
- If its input ends (for example when you pipe moves into it), it loops forever instead of
  exiting, so cap the output when you script it.
