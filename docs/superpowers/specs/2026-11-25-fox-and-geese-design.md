# Fox and Geese (catalogue, sides with their own rules)

The real game that stands in for the made-up Siege: two sides with different pieces and different
rules, one hunting the other. Sources differ on the details (the number of geese, whether the board
has diagonals, how the fox wins), so the choices below are Frank's to overrule.

## The rules as built

- **Board:** the 33-point cross, a 7x7 grid with the four 2x2 corners cut out
  (`disabled_positions`), lines along the rows and columns only.
- **Geese (13):** the three rows nearest them (3, 3 and 7 points). They move first. A goose steps one
  point forward or sideways and never takes anything.
- **Fox (1):** starts in the middle. Steps one point along any line, or jumps a goose to the empty
  point behind it, taking it. A jump that can go on **must** (the same `JUMPING` stage as Checkers).
- **The geese win** when the fox has no step and no jump. **The fox wins** when fewer than four
  geese are left: four is the least that can trap it (at the tip of an arm: three neighbours and
  the point behind the fourth), so with three the game cannot be won. **Geese with no move lose.**
  A position seen three times is a **draw**.

Choices, with the other readings: 13 geese (15, 17 and 18 are the balanced variants; the 13 game
favours the fox); orthogonal moves only (the 17-goose game adds diagonals); no forced capture and no
"huffing"; the fox's win is the arithmetic of the board, where some rule sets name a number of
captures (12 of 17).

## What it needed

- **`REPEATS` in an end check of a game with stages.** The refusal is narrowed: positions are
  recorded when a turn is handed on, so an end check reads them whatever stages the turn went
  through; a test inside a way is still refused. (Go's ko rule, in a way, still has to wait.)
- **The written spec of a board with several disabled squares was not stable**: the set came back
  in whatever order it had been filled. It is written sorted now, so a spec reads back as the
  same text (the read-back test found it).
- **The board draws a cross.** With `"style": "intersections"` the lines were drawn over the whole
  rectangle. A board with places missing now paints its colour under its places only and draws a
  line only between two neighbours that are both on it (`hasHoles`, `neighbourLines` in the board
  model). Boards without holes (Go, Xiangqi) draw as before.

## Checked

Engine: sixteen rule tests (the geese's steps, the fox's steps and jumps, a jump off the cross and
over two geese refused, the jump that goes on, each end, the draw) and the move-count row (770 over
four plies). Session `tests/fox_and_geese.rs`: what each viewer is shown and twelve random games to
their end, saved and resumed after every answer. Played in a browser: the cross, the pieces, a goose
selected with its one destination.
