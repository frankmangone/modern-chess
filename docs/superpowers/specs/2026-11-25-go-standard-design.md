# Go as the standards have it: ko in a game with stages

Status: design, for Frank's rulings. Nothing here is built. It follows the content rulings of
`2026-11-23-remaining-work.md` (section 4): "bottom line is we need to comply with Go's
standards".

## What is wrong today

The catalogue's `go.json` has area scoring, a komi of 7.5, no suicide and a ko rule, but the
players cannot agree which stones are dead: both must pass, and a dead stone has to be captured
first. The agreement exists as a variant in the tests (`crates/session/tests/dead_stones.rs`,
built on `ASK` and `turns.stages`), and that variant **has no ko rule**, because a spec with
`turns.stages` may use `REPEATS` only in an end check. So the two halves of real Go do not yet
go together, and the catalogue cannot take the agreement.

## Which Go

The standard rule sets differ, and the engine can write one family and not the other:

| | Chinese / AGA / Tromp-Taylor | Japanese / Korean |
|---|---|---|
| score | area: stones plus territory (the engine's `score` does it, with `TERRITORY`) | territory plus prisoners, with seki and "no points" rules |
| ko | superko: no position repeats (`NOT REPEATS`; the catalogue has it) | simple ko, with special cases for long cycles |
| suicide | refused (AGA, Chinese) | refused |
| after two passes | the players mark dead stones; a dispute resumes play (AGA, Chinese) | the same, with a more involved dispute rule |

**Recommendation: the area family (AGA-like).** `go.json` is already that: area scoring, komi
7.5, a ko that is situational superko (the position and the player to move), no suicide. Japanese
scoring needs prisoner counts and the seki rule, which the blocks cannot write today, and the
gap is large. What the catalogue lacks to be AGA is the dead-stone agreement, so ko in stages is
the piece to build. Left out on purpose, and listed in the known limitations when it lands: the
AGA pass stone, handicap placement, and the 9x9 komi (7.5 is the 19x19 figure).

## Why `REPEATS` is refused in a way of a staged game

Positions are recorded when a turn is handed on. `REPEATS` in a way asks "does the position this
way leaves, with the turn handed to the next player, appear on record?". That is the right
question exactly when the way **ends the turn**. A way that enters a stage (`STAGE`, `ASK`)
does not hand the turn on, and the ways of a stage run in the middle of a turn, so the question
is about a position that will never be recorded. The check that refuses `REPEATS` in any spec
with stages is a blunt way to keep those cases out. Go's stone rule is not one of them: placing a
stone always ends the turn, even in a game that has a marking stage elsewhere.

## Proposal: allow `REPEATS` where every way that reaches it ends the turn

Refuse it, when the spec is read, only where a way that reaches the test can enter a stage:

1. In an action with a `stage:` (a stage's way), always.
2. In an action that contains a `STAGE` or `ASK` effect anywhere in its steps, `if` branches
   included (conservative: a branch that is never taken still refuses).
3. In a piece's move, if that move contains `STAGE` or `ASK` (Fox and Geese's jumps do).
4. In an `every_action` entry, if some action it applies to (all actions, or those its `for`
   lists) falls under 1 or 2. Go's ko entry already says `"for": ["STONE"]`, so the stone's
   rule passes, and an entry with no `for` in a game with a marking stage is refused with a
   message that says to add one.
5. Unchanged: an end check may always use it.

The error names the effect that makes the way unable to end the turn, so the author knows what
to move. No runtime change: the position a way leaves is already computed with the turn handed
on, which is what a way that ends the turn needs. It is a spec-reader change and its tests.

Considered and rejected: judging `REPEATS` at run time, true only if the way did end the turn
(a rule that silently turns itself off is worse than a refusal); and recording a position at
every stage boundary (it changes what `REPEATS` means for every game that has it).

## Go with the agreement, in the catalogue

With the refusal narrowed, the test variant becomes the real thing:

- `go.json` (and `go_13`, `go_19`) gain `turns.stages` `MARKING` and `REPLY`, the actions of the
  variant, an `AGREED` counter, and keep `NOT REPEATS` in the stone's rule.
- **Marking by group, not by stone.** The variant marks one stone at a time; a real marking
  flags the whole group (`FOR_EACH` over `GROUP`, which the engine has) and lets a marked group be
  revived with the same click. This is the one new rule of the slice, and a page concern: the
  skin draws a `DEAD` stone as a faded stone.
- **After a dispute** the stones are alive again and play resumes with the next player in the
  turn order. (AGA: the player who passed last resumes; with two players the difference is who
  moves first, and it is Frank's ruling below.)
- The dead stones leave the board when the other player agrees, and the area score decides the
  game as today.
- The web needs no new panel: the island offers `MARK`, `DONE`, `RESUME`, `AGREE` and `DISPUTE`
  as actions of the stage, as it does for `ASK` in rock-paper-scissors, and the board click
  answers the `MARK` choice.

## Tests

- Engine: `REPEATS` is refused in a stage's action, in an action that runs `STAGE` or `ASK`, in a
  move that does, and in an unscoped `every_action` entry of a game that has such an action; it is
  allowed in an action that never does, in an entry scoped by `for` to such actions, and in an end
  check. Each refusal has a test that fails without the check.
- Session: the dead-stone variant gets its ko rule back, with a ko position played out (a
  recapture is refused), the agreement played through (`AGREE`, then the score) and the dispute
  (`DISPUTE` or `RESUME`, then play on, and a position from before the passes still cannot
  return). The catalogue Go joins the robustness test.
- Web: the marking flow in the play page, from a stubbed state, as the rock-paper-scissors flow
  was tested.

## For Frank

1. **Which family?** The area family above (recommended), or the Japanese rules, which need new
   blocks (prisoners, seki) and a design of their own.
2. **Who resumes after a dispute?** The next player in the turn order (the engine's natural
   choice), or the player who passed last (AGA). With two players, one of the two.
3. **The 9x9 komi.** Keep 7.5, or a smaller one for the small boards (6.5 is common on 9x9; 5.5 is
   used for beginners)?
4. **Pass stones.** Skip them (the AGA rule counts a pass as a prisoner for the opponent), or
   write them with a counter? They only matter to make area and territory scoring agree, so skipping
   is fine for area scoring.

## Not in this slice

A review of how `PASSES` counts "in a row" after a dispute (a resumed game must not end on one
more pass), and the clock for a player who never answers (already in the known limitations).
