# 1. Your first game

We start with the smallest game that still *plays*. It has one kind of piece that does one thing.
Along the way you will meet the five parts every game is made of.

## The rule

**MARCH** is played on a board three squares wide and five squares tall.

- Each player has three marchers, lined up along their own edge of the board.
- On your turn, move one marcher one square **forward**, onto an empty square.
- Nothing can be captured, and nothing can jump.
- When the player to move has no legal move, the game is a draw.

It is not a good game. It is a good *first* game, because every line of it is visible.

## The spec

This is the whole thing. It is saved as `apps/docs/examples/march.json`.

```json
{{#include ../examples/march.json}}
```

A game spec always has these five parts: **board**, **players**, **turns**, **pieces**, and a
**name** for the game. Let's go through them one at a time.

## The board

```json
"board": { "dimensions": [3, 5] }
```

`dimensions` is `[columns, rows]`: three columns, five rows.

Every square has an address written `[x, y]`. Counting starts at **zero** and the origin is the
**bottom-left** square, so:

```text
     x=0  x=1  x=2
y=4   .    .    .      <- BLACK starts here
y=3   .    .    .
y=2   .    .    .
y=1   .    .    .
y=0   .    .    .      <- WHITE starts here
```

The top-right square is `[2, 4]`. You will use this `[x, y]` form everywhere.

## The players

```json
{
  "name": "WHITE",
  "direction": [[1, 0], [0, 1]],
  "starting_positions": [
    { "piece": "MARCHER", "positions": [[0, 0], [1, 0], [2, 0]] }
  ]
}
```

Each player has three things:

- **`name`** is how the player is referred to everywhere else in the spec. It is only a label,
  so call them whatever you like.
- **`starting_positions`** says which pieces start where. `"piece"` is a piece **code**, and it
  must match a piece you define under `pieces`. `positions` is the list of squares.
- **`direction`** says which way is "forward" for this player. It deserves its own section.

### `direction`: write the moves once

You will describe each piece's moves **once**, as if the piece belonged to a player walking
**up** the board. The engine then turns those moves around for each player using their
`direction`.

For WHITE, forward is up, so the direction is the "do nothing" matrix:

```json
"direction": [[1, 0], [0, 1]]
```

For BLACK, forward is down, so we flip the board around:

```json
"direction": [[-1, 0], [0, -1]]
```

That is why MARCHER has a single move, `[0, 1]` ("one step up"), and still works for both
players. WHITE walks up the board and BLACK walks down it.

The four directions you will use most:

| The player moves... | `direction` |
|---------------------|-------------|
| up the board        | `[[1, 0], [0, 1]]` |
| down the board      | `[[-1, 0], [0, -1]]` |
| to the right        | `[[0, -1], [1, 0]]` |
| to the left         | `[[0, 1], [-1, 0]]` |

You can copy these without understanding the maths behind them. A game with four players simply
uses all four.

## The turns

```json
"turns": { "order": ["WHITE", "BLACK"] }
```

`order` is the list of players, in the order they play. The engine goes around the list again
and again. WHITE goes first because they are first in the list.

## The pieces

```json
{
  "code": "MARCHER",
  "name": "marcher",
  "moves": [
    {
      "id": 0,
      "step": [0, 1],
      "actions": [{ "state": "EMPTY", "action": "MOVE" }]
    }
  ]
}
```

A piece has a **`code`** (its identifier, used in `starting_positions`), an optional **`name`**,
and a list of **`moves`**. Each move is a small recipe with three ingredients:

- **`id`** is a number that is unique within this piece. It does nothing yet. Later chapters use
  it to let one move refer to another.
- **`step`** is how far the piece reaches, as `[dx, dy]`. `[0, 1]` means "no sideways movement, one
  square forward".
- **`actions`** is what the piece does depending on what it finds on that square.

### Reading `actions`

```json
"actions": [{ "state": "EMPTY", "action": "MOVE" }]
```

Read it as a sentence: *"If the square is* **empty**, *the piece can* **move** *there."*

`state` can be `EMPTY`, `ENEMY` or `ALLY`: what is standing on the square the step lands on. A
move is only offered when there is an action for the state the square is actually in. MARCHER only
has an action for `EMPTY`, so:

- an empty square ahead means the move is offered,
- an enemy marcher ahead means no action fits, so it is **blocked**,
- an ally ahead is blocked too.

To let a piece capture, you would add a second action, `{ "state": "ENEMY", "action": "CAPTURE" }`.
We do exactly that in the next chapter.

## What the engine does

With just that, the engine already knows how to:

- show each player only the squares they can legally move to,
- flip every move for BLACK,
- alternate turns,
- and end the game when the player to move is stuck. You did not write a rule for that. With no
  `leader` piece defined, a player with no legal move simply draws the game.

## Play it here

This board runs the real engine, in your browser, on the spec above. Click a marcher, then one of the
highlighted squares. The turn passes to the other side by itself. Notice what you *cannot* do:
the marchers never capture and never sidestep, because the spec gave them exactly one move.

<modern-chess-board spec="boards/march.json">Playable board (needs JavaScript).</modern-chess-board>

## Try it

From the root of the repository:

```sh
cargo run --bin modern-chess-cli apps/docs/examples/march.json
```

The CLI draws the board and asks for a square. Type the address of a piece to see where it can go,
then type one of those squares to move:

```text
Current player: WHITE
Select position to see available moves (e.g., [0, 1]): [0, 0]
Valid moves:
[0, 1]: "MOVE"
Select move to execute (e.g., [0, 1]): [0, 1]
```

Then BLACK plays, and so on. Notice that BLACK's marcher at `[0, 4]` moves **down** to `[0, 3]`
even though its only step is `[0, 1]`. That is `direction` at work.

Keep going until every column is jammed. The game ends with `Stalemate! It's a draw.`

## Now change it

Edit a copy of `march.json` and run it again after each change.

1. **Widen the board.** Make it `[5, 5]` and give each player five marchers. You need to change
   `dimensions` and both `positions` lists. Be careful here: the engine does **not** warn you
   about a starting square that falls outside the board. The piece simply does not show up, so
   if a piece is missing, check its address first.
2. **Let marchers sidestep.** Add two more moves to MARCHER, with `"id": 1` and `"id": 2`. Use
   steps `[1, 0]` and `[-1, 0]` and the same `actions` as before. At the start the sideways
   squares are held by allies, so only a marcher that has stepped forward gets sideways options.
   Columns are no longer jammed for good: marchers can shuffle sideways, so the game is no
   longer forced to end.
3. **Let BLACK go first.** Add `"start_at": 1` to `turns`. It is the position in `order` where
   the game begins, counting from zero. All the fields are listed in `specs/SPEC_FORMAT.md` in
   the repository.

Next, we give the pieces real personalities: pieces that slide, pieces that leap, and pieces that
capture. We will do it by building a real game from India, over a thousand years old.
