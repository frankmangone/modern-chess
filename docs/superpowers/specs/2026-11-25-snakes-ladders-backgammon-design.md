# Snakes and Ladders and Backgammon (catalogue, dice)

Frank asked for both ("along with Snakes and Ladders, could we add Backgammon?"). Both are played by the
blocks that exist (dice, bags, maps, stages); nothing is written for one game. The rulings are
Claude's, for Frank to overrule.

## Snakes and Ladders (built)

- **Board:** a map of 101 places: the start (off the board) and the squares 1 to 100. Each holds two
  tokens. A `step` link joins each square to the next (1 to 2 ... 99 to 100, the numbering a
  boustrophedon on the 10x10 drawing); a `jump` link carries the foot of each ladder up and the head of
  each snake down. The layout is the common one: nine ladders, ten snakes.
- **A turn** is the one action, `ROLL`: the tray is cleared, a die is rolled into it, and the mover's
  token walks that many `step` links, then follows the square's `jump` link once, if it has one.
  Nothing is chosen, so a roll is never seen before it is played (the rule that refuses a choice after
  a `ROLL`).
- **Finish:** the first token on 100 wins, and it needs the exact number: a roll that would pass 100
  leaves the token where it is (the walk cannot be formed, a `COUNT` over it is false, and the `if`
  around it does nothing). The turn still passes.
- Both tokens may share a square and nothing is captured. No extra turn on a six.
- **Tokens** are two codes (`RED_TOKEN`, `BLUE_TOKEN`) because a place that holds many starts with
  nobody's pieces, so the roll branches on `PLAYER`.

### What it needed

- **A skin can draw a map itself.** A map drew dots and lines, so snakes and ladders had no picture
  (a labelled link is not drawn). A skin now may give `board.art`, an inline SVG in the map's units
  (`SPEC_FORMAT.md`, map boards); it replaces the dots, lines and areas and turns with a flipped
  board. Backgammon will use it too.
- Nothing in the engine.

### Checked

Engine: nine tests (the start, every roll walks the right distance, a ladder, a snake, a jump taken
once, both tokens on a square, the exact finish and the overshoot, a ladder to 100, forty games to
their end). Session: the view (101 nodes, one action, no places), the seed never shown, the match saved
and resumed after every roll to the end, other seeds playing other games. The catalogue's move-count
table has a row (0: the action answers nothing). The board was drawn in a browser from the same model
functions.

## Backgammon (proposed, not built)

- **Board:** a map of 24 points (capacity 15 each; a point holds checkers of one side, or none),
  numbered as the standard notation does: White moves from 24 down to 1 and bears off from 1 to 6,
  Black the other way. The bar and each side's tray are bags. Two link labels (`down`, `up`) give each
  side its direction, with a skin's `board.art` drawing the triangles, bar and numbers.
- **Dice:** two dice rolled into the mover's own bag, each face a code `D1` to `D6` (the number rolled
  is the code, not a count of pips, so each die stays itself); a double puts two more of that code. The
  next player's roll clears both bags, so no stale die can be played.
- **A turn:** `ROLL`, then a stage `PLAY` of single-die moves. After each move the turn goes on while a
  die is left and a move exists; when none is, the turn passes by the stage rule.
- **Moves** (one action per side and die value, so a `FOLLOW` has its distance written): enter from the
  bar first; move a checker to a point with no more than one enemy checker; hit a lone enemy checker
  (it goes to the bar); bear off only when all fifteen are in the home board, with the exact die or,
  when no checker stands farther back, a larger one.
- **End:** fifteen checkers off wins.

Left out, as limitations to list: the doubling cube, gammons and backgammons (a game is one win), the
opening roll for who starts (White starts), and the rule that both dice must be played when they can be
(and the larger one when only one can). That rule needs a look ahead of whole turns that the engine
does not have; the other rules are exact.

### For Frank

1. **Scope.** Is "one game, no cube, no gammon, White starts" the Backgammon you want, with the
   both-dice rule listed as a limitation?
2. **Choosing a die.** After picking a checker, the buttons are the dice that checker can play
   (the action names). The alternative is to pick the die first (a code choice).
