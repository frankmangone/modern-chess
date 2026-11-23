# The Flying Knight

Item 4 (first in the order of `2026-11-23-remaining-work.md`). A short design: a small change,
one engine test and one end-check field, plus a catalogue spec.

## Intent

Add the Flying Knight to the catalogue. Rules (chess.com forum, "The Flying Knight", 2014): White
has the full army and one move per turn; Black has a king and a knight on the queen's square and two moves
per turn, both used, by one piece or two; everything else is standard chess. Frank's rulings:
build it only if cheap; Black stalemated is a draw; Black stuck in check is a loss (checkmate).
The thread leaves check across the double move open; the engine design chose the Monster Chess
convention: a double mover may step into check on the first move if the second gets out, and a
king is in check when it can be taken within the opponent's two moves.

## What the look-ahead costs (it is cheap)

Found by reading the engine, not by building:

- **Black's side needs nothing new.** An action may be limited to a player (`players`), and the
  ways of an action are worked out on the board as the steps so far leave it. So Black's turn is
  one action, `DOUBLE_MOVE`, with two `PIECE` + `MOVE` pairs. `every_action` (with `for`, which
  names actions) judges the king's safety on the position the whole action leaves, with no
  look-ahead. (Amended after the build, on Frank's ruling: a king never steps onto a threatened
  square, not even with the first move, so the king's step moves keep a `NOT ATTACKED at TARGET`
  after their `MOVE`, and Black can be stalemated. The first paragraph's "may step into check"
  convention was not kept.)
  Black has at most about 16 moves each time, so at most about 256 ways.
- **White's side needs one test.** White's move is illegal if Black can take the king in its two
  moves. Today `ATTACKED` reads one move of the attacker. It gains `"moves": 2` (default 1): the
  place is attacked if some piece of the attacker threatens it now, or if some piece moves to a
  square it threatens (capturing what stands there) and then some piece of the attacker, on that
  position, threatens it. It uses the same attack sets as `ATTACKED` always did (see "Attack-set
  legality" in the remaining work: conditions on moves are ignored, which does not matter for a
  king and a knight).
- **Mate and stalemate** (end checks) read "stuck and king attacked": for White that must be the
  two-move reading, for Black the one-move reading. End checks have no player filter. They gain
  `"players": [..]` (the viewpoint player must be listed; absent means everyone), as actions and
  moves already have. This is a small slice of step 8, pulled forward because the game needs it.

## Goals

- `flying_knight.json` in the catalogue, with Black stalemate a draw and Black mate a loss,
  White's mate and stalemate as in chess under the two-move reading of check.
- `ATTACKED` `moves: 2`, and `players` on an end check, each documented in `SPEC_FORMAT.md`, with
  tests that fail without them.
- A test of the game: both sides' check rules at the double move (Black steps into check with
  the first move and out with the second: legal; Black ends in check: not offered), White cannot
  move a piece that exposes its king to a two-move capture, and the three results.

## Non-goals and anti-goals

- No look-ahead of more than two moves, no `moves` on any test but `ATTACKED`, no search
  (no "exists a move such that" test). Anti-goal: a rule that reads close but silently differs
  from the one written: `moves` is refused when it is not 1 or 2, and refused with `keeping`.
- No change to any existing spec's meaning (`moves` defaults to 1; `players` defaults to all).
- No reworking of attack sets.

## Approaches

1. **Turn order `["WHITE","BLACK","BLACK"]` and an "exists a move such that" test.** Black's
   first move is legal if some second move is safe. Needs a nested search test judged per
   candidate way, and the stage-1 ways filtered by it: the heavy route, and the new test is wide.
2. **Static threat-only moves** giving each Black piece its two-move reach as extra threatening
   moves. Needs no engine change but is approximate (blocked first squares, the king-then-knight
   combinations), so the spec would say something other than the rule. Rejected.
3. **Chosen: a double-move action, `ATTACKED` with `moves: 2`, and `players` on end checks.**
   The two-move reach is exact for pieces without conditions, and everything else is already
   in the format.

## Left to the builder

The exact message wording for the refusals; whether `moves` is a `u8` or an enum; the catalogue
count tests and the book's variants chapter entry; castling in the spec reads "not through or
out of check" with `moves: 2` on the king's squares (my call, since a square the double mover
can reach in two moves is a square White's king may not cross). Details of the web: the turn has
four choices in a row, as Checkers' chains already do.

## Calls I made (for you to check)

- Black's two moves are one action, so the history records one turn with two moves, not two
  turns of Black. Repetition and the fifty-move counters see Black's turn as one turn.
- `players` on end checks is added for this game now, not left to step 8.
- Date `2026-11-26`, after the unknown-fields design.
