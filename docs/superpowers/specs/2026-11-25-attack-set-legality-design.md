# Attack-set legality: what a piece attacks, from its own moves

Status: design, for Frank's rulings. Nothing here is built.

## The situation

`ATTACKED` (check, castling through fire, the Flying Knight's look-ahead) asks the **attack
map**: the squares a piece could capture on. The map is not worked out from the piece's moves.
It is a second, separate piece of geometry in `MoveBlueprint::visit_threats`:

- follow the move's `step`; if it repeats, go on until an ally (stop before it) or an enemy
  (stop on it), or until the repeat is spent;
- ignore `repeat.until`;
- ignore every test in the move's `do`, except one heuristic: a move whose first test asks for an
  empty `TARGET` threatens nothing (that is how a pawn's push is kept out).

The moves themselves are judged properly: `candidate_targets` follows the real `until`, then
the steps run with all their tests. So a game has **two definitions of "where can this piece go
and take"**, and they agree only for pieces whose moves are plain slides and steps. Where a move
has a condition, they disagree, and check is read from the wrong one.

## What is wrong in the shipped games

Confirmed by tests on the shipped `xiangqi.json`, each with a check on the board and a count of
the legal replies:

- **Cannon, backwards.** Its capture needs exactly one screen. The map ignores that: a cannon with
  nothing between it and the general "gives check" (all replies refused), and one with a lone screen
  does not (every reply allowed, the general left to be taken). Janggi's cannon has the same fault.
- **Horse through a blocked leg.** The leg square is an `offset` test; the map ignores it, so a
  horse attacks through a piece on its leg. With the leg blocked, the general is still read as in
  check.
- **Elephant through its eye**, by the same code path (the eye is a `PATH` test). Not yet tested on
  its own.
- Anything else a spec writes as a condition: a move kept in a zone threatens outside it; a
  slide with its own `until` threatens as if it had the default.

Chess and the games like it are not affected: every condition on their moves is about `TARGET`,
and the one heuristic covers it. The catalogue's moves that carry other conditions are
Xiangqi's horse, elephant, cannon and zone-bound pieces, and Janggi's and Mini Xiangqi's cannons.

The known limitation that says "tests do not affect threats, Xiangqi's flying general relies on
this" is out of date: the flying general is the `threat_only` move, which has no zone test and
is the reason it works. Nothing relies on the zone leaking.

## The principle

A spec is a set of general blocks, and behaviour arises from composing them. The attack map
breaks that: it is a special geometry that a spec author cannot compose with. So it should not be
a separate geometry at all.

> **A piece attacks a square if one of its moves could take there.** The map is the moves, asked
> about a square, with the rest of the position as it is.

No new block and no new key in the spec: the cannon, the horse and the next game's strange piece
get the right attack set from the conditions the author already wrote for the moves.

## Proposed rule

For an attacker's piece at `from` and one of its moves (not restricted away by `players`):

1. **Where it can go.** The move's candidate squares, by the same function that move generation
   uses: the step, the repeat and the real `until` (the default `until`, "a piece on the next
   square", stays as an inline check, so a plain slide costs what it costs today).
2. **Whether it may.** The move's *conditions*, the run of tests at the start of its `do` (before
   its first effect, choice or `if`), judged as the move is judged, on the position where `TARGET`
   holds:
   - the piece that stands there, when one does;
   - when the square is empty, a **ghost**: a piece of the side the question is for, with no
     code. It passes `owner` filters and fails `codes` filters, so a pawn's push (`NOT COUNT at
     TARGET`) does not reach an empty square and its capture does, which is what the empty-target
     heuristic did by hand, and a cannon's "not onto a cannon" test passes.
3. **What does not count.** The attack map is *pseudo-legal*, as it is today: `every_action` is not
   applied (no check rule inside the check rule). Tests that read a way in progress or the attack
   map itself (`REMOVED`, `MOVED`, `REPEATS`, `ATTACKED`) hold, because there is no way and no
   recursion to ask. Counters, flags, `moved`, zones, offsets, paths and counts read the position
   as they always do.
4. **Unchanged:** `threat_only` moves (still counted, never offered), an effect that captures
   somewhere other than `TARGET` (a draughts jump: the attack is its landing square, as today),
   `keeping` and `moves: 2`, which are built on the attack map and inherit the fix.

`ATTACKED` already asks about specific squares (`reaches` stops at the first hit), so the
conditions are judged only for the squares asked about; the full set (`attacked_by`) judges them
all.

## Why not the alternatives

- **A `threatens` filter on each move written by the author** (a second list of conditions): it is
  the same duplication the principle removes, and the next author has to repeat every condition.
- **Run the real move generator with a ghost and read the result** (`piece_move_ways`): exact for
  everything, including jumps, but each `ATTACKED` becomes a full way search per attacker; the
  chess check test would cost many times what it does. The leading-tests rule gets the same answer
  for every move in the catalogue at a fraction of the price, and `capture_is_a_real_threat` already
  uses the full search for the one case that needs it (Atomic).
- **Leave it and document it**: Xiangqi, a shipped game, plays wrongly.

## How to be sure nothing else changes

The change alters which moves are legal in the games it touches, so it needs a proof that it only
does that.

1. **A differential test over the catalogue.** Keep the old geometry as a test-only function. For
   every spec, play random games and compare the old and the new attack set at every position. The
   sets must be **identical** for every game except an allow-list of the pieces named above, and for
   those the differences must be the ones in the table (cannon, horse, elephant). A new difference in
   chess, Shogi or Atomic fails the test.
2. **Pin each fault** with a position that fails today and passes after: cannon with no screen, with
   one screen and with two; horse with the leg open and blocked; elephant with the eye open and
   blocked; Janggi's cannon over a cannon; a slide with a custom `until`; a zone-bound piece.
3. **Cost.** Time the catalogue-count suite and the robustness test before and after. The target is
   no change for the chess family, since their moves take the inline `until` and the conditions are
   judged only at the squares asked about.
4. **Replay.** Xiangqi matches saved on the old rules replay under the new ones: a move once allowed
   by a false check, or refused by one, can differ. Only Xiangqi is in the catalogue, and the project is in development, so this is a note and not a
   migration; the robustness test resumes games
   after every answer, which covers the mechanism.

## Slices

1. The differential harness and the pinned faults, written first and failing (the harness records
   today's sets as the baseline).
2. The rule: candidates from `candidate_targets`, conditions with the ghost, `reaches` and
   `attacked_by` on the new path, the heuristic removed.
3. Documentation (the limitation, `threat_only`'s "important" note, the spec format's section on
   moves), and the Xiangqi cannon and horse in the book if its chapters describe the old behaviour.

## For Frank

1. **The ghost.** An empty square is asked about as if a piece of the other side stood there, with no
   code. Is "no code" right (a test that wants a particular code on the target does not hold for an
   empty square), or should such a test hold, as "could be a king"?
2. **Tests that read a way or the attack map hold.** The alternative is to refuse them in a move's
   leading tests when the spec is read (no catalogue move has one before an effect except castling's
   `ATTACKED`, which is behind its own empty-target test). Hold, or refuse?
3. **Only the leading tests are conditions.** A move that opens with an effect, a choice or an `if`
   has no conditions and threatens everything it reaches. Fine for every move in the catalogue; say
   so if you want the rule stricter.
4. **Matches in progress.** The note above, or a guard?
