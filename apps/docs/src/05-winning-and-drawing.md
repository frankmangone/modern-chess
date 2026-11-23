# 5. Winning and drawing

So far a game could end in two ways: checkmate, or a player with no legal move. This chapter
shows that *every* way a game can end is the same block, written a different way. A game ends
when a test holds, and the test is one you have already met.

The examples are chess variants. Each is almost identical to chess. Only the ending rules
differ, so open each file next to `chess.json` and compare:

| Game | File | How it ends differently |
|------|------|-------------------------|
| King of the Hill | `king_of_the_hill.json` | Walk your king to the centre to win |
| Three-check | `three_check.json` | Check the enemy king three times to win |
| Shatranj | `shatranj.json` | Leave the enemy a bare king to win |
| Racing Kings | `racing_kings.json` | Reach the far rank first. Giving check is forbidden |

## One list: `end_checks`

`end_checks` is a list at the top of the spec. After every turn, once the turn has passed to the
next player, the engine looks at it in order. Each entry is a **test** and an **outcome**. The
first entry whose test holds ends the game (or takes a player out of it), and the entries after
it are not looked at.

```json
"end_checks": [
  { "when": { "test": "REPEATS", "times": 3 }, "then": { "outcome": "DRAW" } }
]
```

This one says: when the position now on the board has appeared three times, the game is a
draw. The `when` is a test like the ones in chapter 4. The `then` is one of four outcomes:

| Outcome | What happens |
|---------|--------------|
| `WIN` | The player the check is about wins. |
| `LOSE` | That player is out. In a two-player game the other player wins; with more players the rest carry on until one is left. |
| `DRAW` | The game ends with no winner. |
| `HIGHEST` | The game ends, and whoever has the best `score` wins (a tie is a draw). Go uses it, below. |

**Whose point of view?** Each entry has an optional `as`. By default it is `MOVER`, the player
who just played, so `ALLY` in the test means the player who just moved. Write `"as": "NEXT"` to
judge the position from the player *about to play*. A player who has no way to play is that
player, so those entries are all `NEXT`.

Everything below is a recipe: one test, one outcome.

## Recipes for a win

### Get there first

King of the Hill (`king_of_the_hill.json`):

```json
{
  "when": { "test": "SOME",
            "in": { "region": "BOARD", "where": { "owner": "ALLY", "codes": ["KING"] } },
            "that": { "test": "IN_ZONE", "at": { "place": "EACH" }, "zone": "CENTER" } },
  "then": { "outcome": "WIN" }
}
```

Read it aloud: *some* place on the board holding one of my kings *is in the zone `CENTER`*. `SOME`
takes a region and a test. It looks at each place of the region in turn, and `EACH` in the test
is the place it is looking at. It holds when at least one place passes. The mover wins.

The zone is declared under `zones`, exactly like `REACH_END` in chapter 3:

```json
{
  "name": "CENTER",
  "places": {
    "WHITE": [[3, 3], [4, 3], [3, 4], [4, 4]],
    "BLACK": [[3, 3], [4, 3], [3, 4], [4, 4]]
  }
}
```

Both players share the same four squares, which is natural for a hill in the middle.
**Racing Kings** works the same way with a zone called `BACK_RANK` (row 7 for both players): the
first king to reach it wins. Because every zone is a separate list *per player*, you could just as
easily give each player a different target, as `REACH_END` does for promotion. A test that names a
zone `zones` does not declare is refused when the spec is read.

### Leave them with nothing

Shatranj (`shatranj.json`), where the SHAH is the king:

```json
{
  "when": { "test": "COUNT", "in": { "region": "BOARD" },
            "where": { "owner": "ENEMY", "except": ["SHAH"] }, "min": 0, "max": 0 },
  "then": { "outcome": "WIN" }
}
```

You met this idea in chapter 3. It counts the enemy's pieces that are *not* the SHAH, and wins when
there are none. `except` is a filter field like `codes`, the other way round: the piece's code is
none of these. Take `except` away and the count is of every enemy piece, so the game ends the
moment an opponent has no pieces at all.

### Count the checks

Three-check (`three_check.json`) needs a number that grows during the game. A **counter** is a
name the game declares, and each player holds a number for it, starting at 0, saved with the
game:

```json
"counters": ["CHECKS", "QUIET"]
```

Two effects change a counter, `ADD` and `RESET`. They are steps like any other, so they live
where every move can reach them, in `every_action`. This entry adds one to the mover's `CHECKS`
whenever the move leaves the enemy king attacked by the mover:

```json
{ "do": [
  { "if": { "test": "ATTACKED",
            "in": { "region": "BOARD", "where": { "owner": "ENEMY", "codes": ["KING"] } },
            "by": "MOVER" },
    "then": [ { "effect": "ADD", "counter": "CHECKS" } ] }
] }
```

The ending is one line: read the counter with the test `COUNTER`.

```json
{ "when": { "test": "COUNTER", "name": "CHECKS", "min": 3 }, "then": { "outcome": "WIN" } }
```

You can see these fire in the CLI with tiny positions. For King of the Hill, put one king a step
from the centre and move it in. For Three-check, a rook giving three checks in a row wins.

### Join your zones

**Hex** (`hex.json`): each player tries to join their two sides of the board with one unbroken
chain of stones. The zones `SIDE_A` and `SIDE_B` are ordinary entries under `zones`, and each one
lists different cells for each player, so `SIDE_A` is the left edge for one player and the bottom
edge for the other. A chain is a `GROUP`, the region from chapter 10: a stone and every stone of
the same player joined to it. The test says that some stone of mine has a group that touches
both sides:

```json
{
  "when": { "test": "SOME",
    "in": { "region": "BOARD", "where": { "owner": "ALLY", "codes": ["STONE"] } },
    "that": { "test": "ALL", "of": [
      { "test": "SOME",
        "in": { "region": "GROUP", "of": { "place": "EACH" }, "through": "adjacent", "where": { "codes": ["STONE"] } },
        "that": { "test": "IN_ZONE", "at": { "place": "EACH" }, "zone": "SIDE_A" } },
      { "test": "SOME",
        "in": { "region": "GROUP", "of": { "place": "EACH" }, "through": "adjacent", "where": { "codes": ["STONE"] } },
        "that": { "test": "IN_ZONE", "at": { "place": "EACH" }, "zone": "SIDE_B" } }
    ] } },
  "then": { "outcome": "WIN" }
}
```

A `SOME` inside a `SOME` makes `EACH` the inner place: the outer one picks a stone, the inner ones
walk its group. `through` says which neighbours join a chain (`"orthogonal"` or `"adjacent"`). To
ask for only some of the sides (Havannah's fork would be 3 of 6), write the combinations in an
`ANY`. A stone sitting on the other player's sides does not count for you.

### Score after both pass

Go has no winner until the players stop. Two passes in a row end it, and the winner is whoever
has the best score (`go.json`):

```json
"score": [
  { "term": "PIECES", "where": { "codes": ["STONE"] } },
  { "term": "TERRITORY", "where": { "codes": ["STONE"] } },
  { "term": "POINTS", "value": 7.5, "to": ["WHITE"] }
],
"end_checks": [
  { "when": { "test": "PASSES", "in_a_row": 2 }, "then": { "outcome": "HIGHEST" } }
]
```

`score` is a list of terms that are added up for each player: their stones, the empty regions only
their stones touch, and 7.5 points for White for moving second. `PASSES` holds when the last two
turns, by anyone, changed nothing. Chapter 10 plays this game.

## Recipes for a draw

A draw is a different animal: nobody wins. Chess ends in `DRAW` three ways, and each is an entry of
the list in `chess.json`.

**Repetition.** The one you saw above: `REPEATS` with `times`.

```json
{ "when": { "test": "REPEATS", "times": 3 }, "then": { "outcome": "DRAW" } }
```

The position is the board, the pieces' state and whose turn it is. Chess uses `3`. Shogi
(`shogi.json`) uses `4`, because its repetition rule needs four occurrences.

**Fifty moves.** This is a counter again, shared by both players. Every turn adds one to
`QUIET`, unless a piece was captured or a pawn moved, which sets it back to 0. A new test and
a new effect do the work:

```json
{ "do": [
  { "if": { "test": "ANY", "of": [
      { "test": "REMOVED", "where": { "owner": "ENEMY" } },
      { "test": "MOVED", "where": { "codes": ["PAWN"] } } ] },
    "then": [ { "effect": "RESET", "counter": "QUIET", "to": "ALL" } ],
    "else": [ { "effect": "ADD", "counter": "QUIET", "to": "ALL" } ] }
] }
```

`MOVED` asks whether the move has moved a piece that matches the filter, so a pawn *step* counts
(a pawn that is only *captured* did not move). `"to": "ALL"` changes every player's counter, since
the rule is shared. The end check reads the number:

```json
{ "when": { "test": "COUNTER", "name": "QUIET", "min": 100 }, "then": { "outcome": "DRAW" } }
```

A hundred turns is fifty moves by each player. The engine does not know what a "pawn" is: you
tell it, with `codes`. If your game has no such piece, count only captures.

**Insufficient material.** Count what is left on each side. In Atomic Chess a lone king against a
lone king is dead; the entry says it for the mover (`ALLY`) and for the other player (`ENEMY`):

```json
{ "when": { "test": "ALL", "of": [
    { "test": "ALL", "of": [
      { "test": "COUNT", "in": { "region": "BOARD" }, "where": { "owner": "ALLY", "codes": ["KING"] }, "min": 1, "max": 1 },
      { "test": "COUNT", "in": { "region": "BOARD" }, "where": { "owner": "ALLY", "except": ["KING"] }, "min": 0, "max": 0 } ] },
    { "test": "ALL", "of": [
      { "test": "COUNT", "in": { "region": "BOARD" }, "where": { "owner": "ENEMY", "codes": ["KING"] }, "min": 1, "max": 1 },
      { "test": "COUNT", "in": { "region": "BOARD" }, "where": { "owner": "ENEMY", "except": ["KING"] }, "min": 0, "max": 0 } ] }
  ] },
  "then": { "outcome": "DRAW" } }
```

Chess adds a bishop or a knight as the one extra piece a side may have, with more `COUNT`s in an
`ANY`. In chess, a lone king against a lone king is a dead draw, and the CLI agrees:

```text
Stalemate! It's a draw.
```

(The CLI uses the same wording for every kind of draw.)

## Running out of moves

A player who cannot move is *stuck*, and what that means depends on the game. The test `STUCK`
holds for the player about to play when they have no way to play. It can be used only in an end
check that is `as` `NEXT`, since that player is the one with no move:

```json
{ "as": "NEXT",
  "when": { "test": "ALL", "of": [
    { "test": "STUCK" },
    { "test": "ATTACKED", "in": { "region": "BOARD", "where": { "owner": "ALLY", "codes": ["KING"] } }, "by": "ENEMY" }
  ] },
  "then": { "outcome": "LOSE" } },
{ "as": "NEXT", "when": { "test": "STUCK" }, "then": { "outcome": "DRAW" } }
```

That pair is chess. The first says: stuck, and my king is attacked: I lose. That is
**checkmate**, and it goes first. The second catches every other way of being stuck: **stalemate**,
a draw. Because the first entry that holds ends the game, the second is only reached when the first
did not hold.

Games where being stuck is simply a loss write one line, and need no king:

```json
{ "as": "NEXT", "when": { "test": "STUCK" }, "then": { "outcome": "LOSE" } }
```

That is what Chaturanga, Shatranj, Shogi, Xiangqi, Checkers and Ataxx use. In four-player chess
`LOSE` takes only the stuck player out, and the others play on.

None of this keeps a king safe. The `every_action` entry from chapter 2 does that, and every chess
variant here has one. An end check only says what a finished position *means*.

## A forbidden move: no check on the opponent

Racing Kings has one more rule: **you may not give check**. Like the check rule of chapter 2 it
is about every move, so it goes in the same place: one more test at the end of the spec's
`every_action` entry.

```json
{ "test": "NOT", "of": { "test": "ATTACKED",
    "in": { "region": "BOARD", "where": { "owner": "ENEMY", "codes": ["KING"] } }, "by": "MOVER" } }
```

It is chapter 2's check test turned around. The region is the squares holding an *enemy* king,
and `"by": "MOVER"` asks whether *your own* pieces attack them once the move is made. If they do,
the move is not offered. Try a copy of `racing_kings.json` with only a rook on `[3, 0]` and an
enemy king on `[7, 7]`. The rook can slide to `[4, 0]`, `[5, 0]` and `[6, 0]`, but `[7, 0]` is
not offered, because from there it would attack the king down the file.

An `every_action` entry applies to every action unless its `for` names some (see the
reference). Racing Kings has one action, `MOVE`, so the rule covers every piece and every move.

## Play it here

**An instant win.** This is King of the Hill with White's king one step from the shaded centre.
Click the king and step onto the hill: the game ends at once with a win. That is one entry in
`end_checks`, judged before checkmate or stalemate.

<modern-chess-board spec="boards/hill.json" skin="boards/chess.skin.json">Playable board (needs JavaScript).</modern-chess-board>

**Running out of moves.** Ordinary chess here, with White to move. Find the queen move that leaves
Black with no legal move while the king is *not* in check. Chess scores that as a draw. (Hint: the
queen goes straight up the board, two squares.) With a stuck player losing, as in Chaturanga, the
same position would be a win instead.

<modern-chess-board spec="boards/stalemate.json" skin="boards/chess.skin.json">Playable board (needs JavaScript).</modern-chess-board>

## Now change it

1. **A smaller hill.** In `king_of_the_hill.json`, edit `CENTER` so that it holds only
   `[4, 4]` for both players. The hill is now a single square.
2. **Two checks.** Change `"min": 3` to `2` in the `CHECKS` entry of `three_check.json`'s
   `end_checks`.
3. **No draws by repetition.** Remove the `REPEATS` entry from `three_check.json`'s
   `end_checks`. The other draw rules still work, but repeating moves is no longer a draw.
4. **Losing by being stuck.** Add `end_checks` with the one `STUCK` line above to
   `apps/docs/examples/march.json`, and play it to the jam from chapter 1. Compare the ending
   with a game that has no end check at all.

Next, a game where the *shape* of the board and the pieces' freedom both matter: Xiangqi.
