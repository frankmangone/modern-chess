# 10. Placing stones: placing, liberties and counting

Every game so far moved pieces that were already on the board, or dropped pieces that had been
captured. **Go** does neither. The board starts empty, each turn a player places a new stone,
stones are captured by being surrounded, and the game ends when both players pass and the board
is counted.

The spec is `crates/engine/specs/go.json`. The players are BLACK and WHITE, the board is 9×9,
and a stone sits on the crossing of two lines rather than inside a square. (`go_13.json` and
`go_19.json` are the same spec on bigger boards.) (How that is
*drawn* is the job of a skin, not the spec. See the board skins in the app.)

None of the rules below is about Go. Each one is a block or a switch in the spec that another game
could use, which is how this chapter is organised.

## The rules we are building

- A turn is **placing a stone** on any empty point, or **passing**.
- A group of stones with no empty point beside it is **captured**.
- You may not place a stone that would leave your own group with no empty point beside it,
  unless the stone captures something first.
- You may not play a move that brings back an earlier position (the **ko** rule).
- When both players pass in a row, each player's score is their stones plus the empty regions
  only they touch. WHITE gets 7.5 extra points, and the higher score wins.

## Placing a stone

In chapter 7 a piece could be dropped only if it was in the hand. A Go player never runs out
of stones, so there is no hand to take them from. A turn that makes a stone is an action with a
`PLACE` effect:

```json
{
  "name": "STONE",
  "do": [
    { "choose": "PLACE", "in": { "region": "BOARD" } },
    { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } } } },
    { "effect": "PLACE", "at": { "place": "TARGET" }, "code": "STONE" }
  ]
}
```

It reads like the drop in chapter 7, with no `HAND` choice:

- **`{ "choose": "PLACE", "in": { "region": "BOARD" } }`** asks where. The `BOARD` region is every
  place of the board, and `TARGET` is the one picked.
- **The test** asks for an empty point. `PLACE` replaces whatever stands on its place, so without
  this test a stone could be put on top of another.
- **The `PLACE` effect** creates a new STONE on the target, owned by whoever moves. The effect
  names the code, so nothing has to be in a hand first and nothing leaves it.

Any tests can be added after the choice. A game where you may place a piece only beside another
of yours would say so with a test, as the drop rules in chapter 7 do. `hand_enabled` is separate:
Shogi has a hand and a drop, Go has a placing and no hand, and a game may have both.

The stone has no moves, so `moves` is an empty list. Both players start with no pieces at all:
`starting_positions` is `[]`.

If a game has two placing actions, say `STONE` and `DISC`, both ask for a place. The player says which
one they mean by naming the action, before or after clicking the point.

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
{ "name": "PASS", "do": [] }
```

```json
"win_conditions": [
  { "type": "AREA_SCORE", "pieces": ["STONE"], "territory": true, "bonus": { "WHITE": 7.5 } }
]
```

A pass is an action with nothing in its `do`. It asks no questions and changes nothing, so naming
it is the whole turn. A pass is recorded in the history, and since a player can always pass, this
game never ends in a stalemate. In the CLI it is one of the `Actions`:

```text
Current player: BLACK
Actions: STONE, PASS (plays)
```

(`PASS (plays)` plays at once. `STONE` only says that the answer is the stone action, which a click
on a point does anyway.)

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

- It offers a placement on every empty point where it is legal, and each placement creates its
  own stone, so there is no supply to run out.
- It removes shut-in groups after each move, and refuses a suicide or a repeated position, so a
  point that would do either is not offered at all.
- It records captured stones like any other captured piece, so the captured counts in the app
  work for Go too.
- It ends the game after the passes, counts the board, and names the winner, or a draw.

## Try it

Click a point to place a stone. Surround one of the other colour to capture it. Try the corner:
a stone there has only two liberties. **Pass** gives up your turn, and the line below the board
shows the count as if the game ended at that moment.

<modern-chess-board spec="boards/go.json" skin="boards/go.skin.json">Playable board (needs JavaScript).</modern-chess-board>

You will notice that a placed stone can sit anywhere, that a point that would be suicide does not
react when you click it, and that the count swings: with one stone on the board, the whole empty
board touches only that stone, so it all counts for its owner.

## Now change it

1. Change `dimensions` to `[13, 13]`, or to `[19, 19]`. Nothing else in the spec mentions the
   size.
2. Set `"allow_suicide": true` and place a stone into a point that is surrounded. Watch it
   disappear.
3. Remove `"territory": true`. Now two passes at the start of a game are a win for WHITE
   by the bonus alone, and a game is won by having more stones.
4. Set `"forbid_repeated_positions": false` and try to build a ko. The recapture is now allowed,
   and you can go round in circles.
