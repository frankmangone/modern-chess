# 10. Placing stones: placing, liberties and counting

Every game so far moved pieces that were already on the board, or dropped pieces that had been
captured. **Go** does neither. The board starts empty, each turn a player places a new stone,
stones are captured by being surrounded, and the game ends when both players pass and the board
is counted.

The spec is `crates/engine/specs/go.json`. The players are BLACK and WHITE, the board is 9×9,
and a stone sits on the crossing of two lines rather than inside a square. (`go_13.json` and
`go_19.json` are the same spec on bigger boards.) (How that is
*drawn* is the job of a skin, not the spec. See the board skins in the app.)

None of the rules below is about Go. Each one is made of blocks in the spec that another game
could use, which is how this chapter is organised.

## The rules we are building

- A turn is **placing a stone** on any empty point, or **passing**.
- A group of stones with no empty point beside it is **captured**.
- You may not place a stone that would leave your own group with no empty point beside it,
  unless the stone captures something first.
- You may not play a move that brings back an earlier position (the **ko** rule).
- When both players pass in a row, they mark the stones that are dead and agree on them. Then
  each player's score is their stones plus the empty regions only they touch. WHITE gets komi (6.5
  extra points on the 9x9 board, 7.5 on the larger ones), and the higher score wins.

## Placing a stone

In chapter 7 a piece could be dropped only if it was in the hand. A Go player never runs out
of stones, so there is no bag to take them from. A turn that makes a stone is an action with a
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

It reads like the drop in chapter 7, with no `HELD` choice:

- **`{ "choose": "PLACE", "in": { "region": "BOARD" } }`** asks where. The `BOARD` region is every
  place of the board, and `TARGET` is the one picked.
- **The test** asks for an empty point. `PLACE` replaces whatever stands on its place, so without
  this test a stone could be put on top of another.
- **The `PLACE` effect** creates a new STONE on the target, owned by whoever moves. The effect
  names the code, so nothing has to be in a bag first and nothing leaves it.

Any tests can be added after the choice. A game where you may place a piece only beside another
of yours would say so with a test, as the drop rules in chapter 7 do. A bag is separate:
Shogi has a hand and a drop, Go has a placing and no bag, and a game may have both. (A game
that takes its pieces from a limited supply writes the bag, and chapter 12 shows how.)

The stone has no moves, so `moves` is an empty list. Both players start with no pieces at all:
`starting_positions` is `[]`.

If a game has two placing actions, say `STONE` and `DISC`, both ask for a place. The player says which
one they mean by naming the action, before or after clicking the point.

## Capturing by liberties

Stones of one player that touch along a row or a column form a **group**. A diagonal does not
connect them. The empty points beside a group are its **liberties**. A group with none left is
captured, and you may not leave your own group with none.

That is a rule about every stone placed, so it goes in `every_action`, the list of steps that run
after an action's own (chapter 2 used it for check). This entry has a `for`: it runs only after
`STONE`, never after a pass. Its steps, one at a time:

```json
"every_action": [
  { "for": ["STONE"], "do": [
    ...the capture...,
    ...the suicide test...,
    { "test": "NOT", "of": { "test": "REPEATS" } }
  ] }
]
```

### Two new regions and a filter

A group and its liberties are regions, so the steps can look at them:

```json
{ "region": "GROUP", "of": { "place": "TARGET" }, "through": "orthogonal", "where": { "codes": ["STONE"] } }
```

**`GROUP`** is the piece at `of` and every piece of the same owner joined to it, neighbour by
neighbour, through `through` (as in `AROUND`), as long as each passes the filter. Here: the stone
just placed and every stone of yours connected to it along rows and columns. If `of` holds no
such piece, the group is empty.

```json
{ "test": "COUNT",
  "in": { "region": "AROUND", "of": { ...the GROUP above... }, "through": "orthogonal" },
  "where": { "empty": true } }
```

**`AROUND`** you have met around one place. Its `of` may also be a region: then it is every place
next to the region and not in it. Around a group, that is the points touching the group.

**`{ "empty": true }`** is a filter for places rather than pieces: a place passes when nothing
stands on it. A `COUNT` with it counts empty places, so this test reads *"the group has at least
one liberty"*. It is written alone, with no `owner` or `codes` beside it.

### The capture

```json
{ "effect": "FOR_EACH",
  "in": { "region": "AROUND", "of": { "place": "TARGET" }, "through": "orthogonal" },
  "do": [
    { "if": { "test": "ALL", "of": [
        { "test": "COUNT", "in": { "region": "AT", "at": { "place": "EACH" } },
          "where": { "owner": "ENEMY", "codes": ["STONE"] } },
        { "test": "COUNT",
          "in": { "region": "AROUND", "through": "orthogonal",
                  "of": { "region": "GROUP", "of": { "place": "EACH" }, "through": "orthogonal", "where": { "codes": ["STONE"] } } },
          "where": { "empty": true }, "min": 0, "max": 0 } ] },
      "then": [
        { "effect": "FOR_EACH",
          "in": { "region": "GROUP", "of": { "place": "EACH" }, "through": "orthogonal", "where": { "codes": ["STONE"] } },
          "do": [ { "effect": "REMOVE", "at": { "place": "EACH" } } ] } ] }
  ] }
```

For each point next to the stone just placed: if it holds an enemy stone, and that stone's group
has no liberty left (`"min": 0, "max": 0`), remove every stone of the group. The second
`FOR_EACH` sits inside the first. Inside it, `EACH` is the stone being removed; outside it,
`EACH` is the neighbour being looked at again.

### The suicide test

```json
{ "test": "COUNT",
  "in": { "region": "AROUND", "through": "orthogonal",
          "of": { "region": "GROUP", "of": { "place": "TARGET" }, "through": "orthogonal", "where": { "codes": ["STONE"] } } },
  "where": { "empty": true } }
```

Then your own group must have a liberty. A placement that leaves it with none is a **suicide**,
and the test fails, so the point is not offered.

The order matters. The test comes after the capture, so it sees the points the capture just
emptied. A stone placed on a point with no empty neighbour is legal when it takes the last
liberty of an enemy group, because the captured stones free the points around it.

Only stones are joined, counted and captured. A piece with another code is neither captured nor
an empty place, so it acts like the edge of the board.

## Ko

```json
{ "test": "NOT", "of": { "test": "REPEATS" } }
```

In the shape known as *ko*, two players could capture and recapture one stone for ever. The last
step of the entry stops it. **`REPEATS`** holds when the board, as the steps so far leave it and
with the turn handed on, is one that an earlier turn left behind. The engine already keeps the
positions each turn left behind. `NOT` turns it into a rule: no move may bring back an earlier
position. It is not the same as the end check `REPEATS` with `times` (chapter 5): that one lets the
repetition happen and declares a draw, this one removes the move.

## Passing, and counting

```json
{ "name": "PASS", "do": [] }
```

```json
"score": [
  { "term": "PIECES", "where": { "codes": ["STONE"] } },
  { "term": "TERRITORY", "where": { "codes": ["STONE"] } },
  { "term": "POINTS", "value": 6.5, "to": ["WHITE"] }
],
"end_checks": [
  { "when": { "test": "COUNTER", "name": "AGREED", "min": 1 }, "then": { "outcome": "HIGHEST" } }
]
```

A pass is an action with nothing in its `do`. It asks no questions and changes nothing, so naming
it is the whole turn. (The catalogue's Go gives its `PASS` one step more, which starts the marking
below; the pass itself is still nothing.) A pass is recorded in the history, and since a player can
always pass, this game never ends in a stalemate. In the CLI it is one of the `Actions`:

```text
Current player: BLACK
Actions: STONE, PASS (plays)
```

(`PASS (plays)` plays at once. `STONE` only says that the answer is the stone action, which a click
on a point does anyway.)

Passing does not end the game by itself. The end check does: it holds once the players have
agreed on the dead stones (next section). Its outcome, `HIGHEST`, counts the board with `score`
and gives the game to the best. (A simpler Go ends on `PASSES`, which holds when the last
`in_a_row` turns, by anyone, changed nothing: with `2` the game ends when both players pass in a
row.) `score` is a list of terms that are added up for each player:

- `PIECES` is one point for each piece of the player's that matches the filter.
- `TERRITORY` counts an empty region for a player when every piece of the filter around it
  belongs to them. A region that touches two players, or nobody, counts for no one. This is the
  one block only Go uses.
- `POINTS` is a fixed number for the players named. Go calls this *komi*, and compensates WHITE
  for moving second. Because it ends in `.5`, the game cannot be a tie.

The player with the highest score wins. Leave out `TERRITORY` and only the stones are counted,
which is how Reversi is scored.

### Agreeing which stones are dead

The count is of the board as it stands, so a dead stone has to leave it first. Real Go lets the
players say so, and the spec does it with stages (chapter 9):

- The second pass in a row runs `STAGE` `MARKING`: the player who passed last keeps the turn, in a
  stage whose actions are `MARK`, `DONE` and `RESUME`.
- `MARK` asks for a stone and turns it, and the whole group it belongs to, into a `DEAD` piece
  (`FOR_EACH` over a `GROUP`); marking a dead stone gives its group back. It stays in the stage, so
  the player marks as many groups as they like. The page draws a `DEAD` stone pale.
- `DONE` runs `ASK`: the other player answers in the stage `REPLY`, with `AGREE` (the dead stones
  are taken off, a counter `AGREED` is set and the end check ends the game) or `DISPUTE` (the
  stones are alive again and play goes on with the next player).
- `RESUME` is the marker changing their mind: the stones are alive again and the turn is handed on.

The ko rule above sits in an `every_action` entry with `for: ["STONE"]`. The game has stages, and
a rule that reads positions needs a way that ends the turn, which placing a stone does and the
actions of the marking stage do not (see the limits at the end of the book).

## What the engine does

- It offers a placement on every empty point where it is legal, and each placement creates its
  own stone, so there is no supply to run out.
- It runs the `every_action` entry after each stone: it removes shut-in groups, and refuses a
  suicide or a repeated position, so a point that would do either is not offered at all.
- It records captured stones like any other captured piece, so the captured counts in the app
  work for Go too.
- It ends the game after the passes (the end check), counts the board with `score`, and names the winner, or a draw.

## Play it here

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
2. **Allow suicide.** Replace the suicide test with an `if` that removes your group instead of
   refusing the move:

   ```json
   { "if": { "test": "NOT", "of": { ...the suicide test... } },
     "then": [
       { "effect": "FOR_EACH",
         "in": { "region": "GROUP", "of": { "place": "TARGET" }, "through": "orthogonal", "where": { "codes": ["STONE"] } },
         "do": [ { "effect": "REMOVE", "at": { "place": "EACH" } } ] } ] }
   ```

   Place a stone into a point that is surrounded. Watch it, and the group it joined, disappear.
3. Remove the `TERRITORY` term from `score`. Now two passes at the start of a game are a win for WHITE
   by the bonus alone, and a game is won by having more stones.
4. Delete the `NOT` `REPEATS` step and try to build a ko. The recapture is now allowed, and you
   can go round in circles.

Next: boards that are not grids at all, only places and the links between them.
