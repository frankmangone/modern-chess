# 10. Placing stones: a supply, liberties and counting

Every game so far moved pieces that were already on the board, or dropped pieces that had been
captured. **Go** does neither. The board starts empty, each turn a player places a new stone,
stones are captured by being surrounded, and the game ends when both players pass and the board
is counted.

The spec is `crates/engine/specs/go.json`. The players are BLACK and WHITE, the board is 9×9,
and a stone sits on the crossing of two lines rather than inside a square. (How that is
*drawn* is the job of a skin, not the spec. See the board skins in the app.)

None of the rules below is about Go. Each one is a switch in the spec that another game could
use, which is how this chapter is organised.

## The rules we are building

- A turn is **placing a stone** on any empty point, or **passing**.
- A group of stones with no empty point beside it is **captured**.
- You may not place a stone that would leave your own group with no empty point beside it,
  unless the stone captures something first.
- You may not play a move that brings back an earlier position (the **ko** rule).
- When both players pass in a row, each player's score is their stones plus the empty regions
  only they touch. WHITE gets 7.5 extra points, and the higher score wins.

## A supply instead of a hand

In chapter 7 a piece could be dropped only if it was in the hand. A Go player never runs out
of stones, so the piece gets a different switch:

```json
{ "code": "STONE", "name": "stone", "unlimited_supply": true, "moves": [] }
```

With `unlimited_supply`, any player may place that piece on any empty square at any time. It is
the same *drop* as before, so a piece can still carry `drop_restrictions`, but nothing has to be
in a hand first and nothing leaves it. `hand_enabled` is separate: Shogi has a hand and no
supply, Go has a supply and no hand, and a game may have both.

The stone has no moves, so `moves` is an empty list. Both players start with no pieces at all:
`starting_positions` is `[]`.

## Capturing by liberties

```json
"liberty_capture": { "pieces": ["STONE"] }
```

Stones of one player that touch along a row or a column form a **group**. A diagonal does not
connect them. The empty points beside a group are its **liberties**. After every move:

1. Each group of the *other* player with no liberties left is removed, all together.
2. Then the mover's own groups are checked. One with no liberties is a **suicide**, and the move
   is not offered.

The order matters. A stone placed on a point with no empty neighbour is legal when it takes
the last liberty of an enemy group, because the captured stones free the points around it.

`allow_suicide` is `false` by default. Set it to `true` and a suicidal move is allowed, and
removes the mover's own group.

The pieces named in `pieces` are the only ones that form groups. A piece with another code is
neither captured nor counted as a liberty, so it acts like the edge of the board.

## Ko

```json
"forbid_repeated_positions": true
```

In the shape known as *ko*, two players could capture and recapture one stone for ever. This
switch refuses any move that would leave the board, with the same player to move, as it was
after an earlier turn. The engine already keeps the positions each turn left behind. It is not the same as `draw_conditions.repetition_count`: that one lets the repetition
happen and declares a draw, this one removes the move.

## Passing, and counting

```json
"allow_pass": true,
"win_conditions": [
  { "type": "AREA_SCORE", "pieces": ["STONE"], "territory": true, "bonus": { "WHITE": 7.5 } }
]
```

With `allow_pass`, the player to move can give up the turn. A pass is recorded in the history,
and since a player can always pass, this game never ends in a stalemate.

Passing does not end the game by itself. `AREA_SCORE` does: when every player has passed in a
row (`after_passes` changes the count), the board is counted.

- Each counted piece is one point for its owner.
- With `territory`, an empty region counts for a player when every counted piece around it
  belongs to them. A region that touches two players, or nobody, counts for no one.
- `bonus` is added to the player named. Go calls this *komi*, and compensates WHITE for moving
  second. Because it is `7.5`, the game cannot be a tie.

The player with the highest score wins. Leave out `territory` and only the stones are counted,
which is how Reversi is scored.

One thing the engine does **not** do is let the players agree which stones are dead. The count
is of the board as it stands, so a dead stone has to be captured in play before passing.

## What the engine does

- It offers a placement on every empty point where it is legal, and it takes the stone from a
  supply that cannot run out.
- It removes shut-in groups after each move, and refuses a suicide or a repeated position, so a
  point that would do either is not offered at all.
- It records captured stones like any other captured piece, so the captured counts in the app
  work for Go too.
- It ends the game after the passes, counts the board, and names the winner, or a draw.

## Now change it

1. Change `dimensions` to `[13, 13]`, or to `[19, 19]`. Nothing else in the spec mentions the
   size.
2. Set `"allow_suicide": true` and place a stone into a point that is surrounded. Watch it
   disappear.
3. Remove `"territory": true`. Now two passes at the start of a game are a win for WHITE
   by the bonus alone, and a game is won by having more stones.
4. Set `"forbid_repeated_positions": false` and try to build a ko. The recapture is now allowed,
   and you can go round in circles.
