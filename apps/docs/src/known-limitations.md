# Known limitations

Writing this guide meant trying every example against the engine. This page lists what
turned up. It describes how things behave **as of November 2026**. Some of these are rough
edges that will probably be fixed, so check them against the current engine before relying on
them.

## Rules the engine does not enforce yet

**Your own leader can be left exposed.** The engine offers moves that leave your own leader
attacked. Only two things guard against it:

- the leader's own moves, through a `NOT_ATTACKED` condition (chapter 2), and
- the end-of-game check, which treats a move that leaves the leader attacked as not counting
  when it decides whether you have *any* legal move.

In practice this means a **pinned piece** (one shielding its own leader) is still offered moves
that expose the leader, and in Xiangqi a general may step onto an open file facing the other
general. If you design a game where this matters, expect it to be your players' job for now.
(Drops are different: the engine never offers a drop that would leave your leader in check.)

## Rules that the shipped specs get wrong, or partly wrong

These are in the specs, not in the engine, and you can fix them in your own copies.

- **Xiangqi cannon (`xiangqi.json`).** The capture needs one piece as a screen, but the screen
  can only be a **friendly** piece. An enemy piece cannot serve as the screen, though real
  Xiangqi allows it. This comes from `"until": "ENEMY"`, which stops the slide at the first
  enemy. See chapter 6.
- **Castling (`chess.json`).** Queenside castling does not check that the square beside the
  rook is empty: a knight on `b1` does not stop it. And a king can castle when its rook is gone
  from the corner. Chapter 4 shows why: the spec describes both players' castling with the
  same neutral offsets, so some corners of the rules are hard to express.
- **Ataxx (`ataxx.json`).** Only the clone moves convert neighbouring stones. In real Ataxx a
  jump converts them too. Chapter 8 has an exercise that fixes it.

## Sharp edges when writing a spec

- **Positions off the board** in `starting_positions` are ignored without warning. The piece
  just does not appear.
- **Unknown condition names** make the move unavailable, without an error. This is deliberate
  (a mistake should block a move, never allow one), but a typo is silent.
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
