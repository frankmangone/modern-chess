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

## Backgammon (built)

Frank's go: "backgammon is a low-hanging fruit"; the two questions below were settled as proposed
(one game, no cube, no gammon, White starts, the both-dice rule listed as a limitation), and the
choice of a die went a different way (see "What changed").

- **Board:** a map of 24 points (capacity 15 each; a point holds checkers of one side, or none),
  numbered as the standard notation does: White moves from 24 down to 1 and bears off from 1 to 6,
  Black the other way. A tray place for each side, off the points (`OFFW`, `OFFB`), where a
  checker is borne off. A link label for each die and direction (`w1`..`w6`, `b1`..`b6`). The skin's
  `board.art` draws the triangles, the bar, the numbers and the trays.
- **Checkers** are two codes, so they stay nobody's; the two bars are shared bags (`White bar`,
  `Black bar`). A player's own bag would make an entering checker theirs, and the point would show
  two rows of the same checker (found by a view test).
- **Dice:** two dice rolled into the mover's own `DICE` bag, each face a code `D1` to `D6`; a double
  puts two more of that code (an `if` for each face). Both players' bags are cleared by the roll.
- **A turn:** `ROLL`, which enters the stage `PLAY`; every move of the stage enters it again, and
  the turn is handed on when no die is left or none has a move.
- **Moves,** one action for each die: `MOVE_n`, `ENTER_n` (from the bar, to the zone `ENTRYn`),
  `OFF_n` (to the tray). `MOVE_n` needs the bar empty, a point with at most one enemy checker,
  hits a lone one (it goes to its owner's bar). `OFF_n` needs every checker in the home board and
  the bar empty, and takes the checker on the point of the die's number or, when none stands farther
  back, the highest one.
- **End:** fifteen checkers in the tray wins (an end check over the tray places).
- 18 actions, 246 links and a 300 KB generated spec: `tools/gen_backgammon_spec.py` writes it, and
  `tools/gen_backgammon_page.py` the page and the board.

Left out, as limitations (listed in the book): the doubling cube, gammons and backgammons (a game
is one win), the opening roll for who starts (White starts), and the rule that both dice must be
played when they can (and the larger one when only one can). That rule needs a look ahead of whole
turns that the engine does not have; the other rules are exact.

### What changed from the proposal

- **A move is chosen by its destination, not by its checker.** `PIECE` picks pieces on the board and
  not the counts a holder keeps, so a point cannot be picked that way; and a single `PLACE` choice
  is all a way may have of a place (a second one would set `TARGET` again). The destination is the
  one that says the most: the checker is the one the die's reverse label reaches from it (for a 3,
  the checker three points farther back), so a click on a point moves a checker there. When two dice
  can reach it, the action is asked (the buttons are the dice: "Move 3", "Move 5"), which answers
  the "choose a die" question the other way round: the die is asked only when it matters.
- **The tray is a place** (not a bag) so that a click can name it, and bearing off is a move to it.
  The zone `SAFE` (the home board and the tray) is what "every checker is home" reads.
- **`hint` in a skin.** The island's line "Place a piece on an empty point" is wrong for a game whose
  points are where a die takes a checker; a skin may now say what the places are for.
- Nothing in the engine.

### Checked

Engine (16 tests): the start, a roll's two dice spent one by one, a double as four moves, a closed
point, a hit, the bar entering first and a blocked entry losing the turn, bearing off (needs every
checker home, the exact number, a larger die only from the farthest point, not while one stands
farther back), the winner, Black's way, a lost die, and six games played to their end with a winner
each. Session: the view (26 places, the bags), the seed never shown, six answers of a match saved
and resumed after every answer to the end, and no kind of checker listed twice on a point. The
board was drawn in a browser from the same model functions.
