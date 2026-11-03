# 2. Pieces that move

In the last chapter our marcher took one step and could not capture. Real games need pieces with
personality: pieces that slide across the board, pieces that leap, pieces that capture, and one
piece that must be protected at all costs.

We will build them from **Chaturanga**, the Indian ancestor of chess, more than a thousand years
old. Its pieces are simple enough to read at a glance and still cover everything in this chapter.

## The rule

Each side has five kinds of piece, and they all capture by moving onto an enemy:

| Piece | Chess equivalent | Moves |
|-------|------------------|-------|
| **RAJA** | king | one square in any of the 8 directions |
| **MANTRI** | (a weak queen) | one square diagonally |
| **RATHA** | rook | any distance, straight along a row or column |
| **GAJA** | (a short bishop) | leaps exactly two squares diagonally |
| **ASHVA** | knight | leaps in an L-shape |

If your RAJA is attacked and you cannot save it, you lose.

The real Chaturanga also has foot soldiers, which promote. We leave them out for now and add them
back in the next chapter. The spec for this chapter is `apps/docs/examples/chaturanga_pieces.json`.
The board, players and turns are written exactly as in MARCH, so we will not repeat them. The
only differences are a bigger board (`[8, 8]`) and five piece types per player.

## Capturing

In chapter 1, a marcher had one action: move onto an empty square. A piece that captures needs a
second one:

```json
"actions": [
  { "state": "EMPTY", "action": "MOVE" },
  { "state": "ENEMY", "action": "CAPTURE" }
]
```

Read it as: *"If the square is empty, move there. If an enemy is standing there, capture it."*
There is no action for `ALLY`, so a piece can never land on a square held by a friend.

Almost every piece in Chaturanga uses exactly these two actions.

## Leapers: one step, one move

The ASHVA moves like a chess knight. It has eight possible landing squares, and each one is a
separate move:

```json
{ "id": 0, "step": [ 2,  1], "actions": [ ...move or capture... ] },
{ "id": 1, "step": [ 2, -1], "actions": [ ...move or capture... ] },
{ "id": 2, "step": [-2,  1], "actions": [ ...move or capture... ] },
{ "id": 3, "step": [-2, -1], "actions": [ ...move or capture... ] },
{ "id": 4, "step": [ 1,  2], "actions": [ ...move or capture... ] },
{ "id": 5, "step": [ 1, -2], "actions": [ ...move or capture... ] },
{ "id": 6, "step": [-1,  2], "actions": [ ...move or capture... ] },
{ "id": 7, "step": [-1, -2], "actions": [ ...move or capture... ] }
```

(In the real file every `actions` list is written out in full. We shortened it here.)

A few things to notice:

- **One direction, one move.** There is no "all directions" shortcut: a piece that can go in
  eight directions has eight entries. It is a little long, but there is nothing to learn and
  nothing hidden.
- **A step is a jump.** The engine looks only at the square where the step *lands*. It does not
  care what stands in between. That is why a knight can hop over other pieces, and why the GAJA
  (steps like `[2, 2]`) can jump over a piece standing on the diagonal square in the middle.
- **Squares off the board are ignored.** A knight in the corner simply has fewer moves.
- **You do not have to worry about direction here.** These pieces move the same in every
  direction, so flipping the board for BLACK changes nothing. The `direction` matrix only matters
  for pieces with a "forward", such as the marcher. Chapter 3 gets there.

## Sliders: `repeat`

A rook does not take one step. It keeps going. To say that, add a `repeat` to the move:

```json
{
  "id": 2,
  "step": [0, 1],
  "actions": [
    { "state": "EMPTY", "action": "MOVE" },
    { "state": "ENEMY", "action": "CAPTURE" }
  ],
  "repeat": { "loop": true }
}
```

This reads: *"Step up. If you can, step up again, and again, as far as you can go."* The engine
offers every square along the way. It stops when:

- it reaches the **edge** of the board,
- it reaches an **ally**. That square is not offered, and neither is anything beyond it,
- it reaches an **enemy**. That square *is* offered, as a capture, but nothing beyond it is.

The RATHA has four of these, one for each straight direction (`[1, 0]`, `[-1, 0]`, `[0, 1]`
and `[0, -1]`).

You can also give a slide a limit. `"repeat": { "times": 3 }` goes at most three squares:

```json
"repeat": { "times": 3 }
```

A RATHA with that limit, standing at `[0, 0]` with a clear column, can reach `[0, 1]`, `[0, 2]`
and `[0, 3]`.

## The leader: protecting your RAJA

So far a piece is only a piece. A game like chess adds one that matters more than the rest. At
the top of the spec:

```json
"leader": ["RAJA"]
```

The `leader` list names the piece (or pieces) that each player must protect. Once you set it, the
engine understands three new things without any more rules from you:

- **Check.** Your leader is *in check* when an enemy piece could capture it on its next move. The
  engine works that out from the very same `moves` you already wrote. There is no separate list
  of "attack rules".
- **Checkmate.** When you have no move that leaves your leader safe, **and** you are in check,
  you lose. The engine tries every move you have and throws away the ones that leave your leader
  attacked.
- **Stalemate.** When you have no safe move but you are *not* in check, the game is a draw.

This is the end-of-game rule from chapter 1, made smarter. Without a `leader`, a player with no
moves just draws the game. With one, the engine can tell the difference between "stuck" and
"checkmated".

### Keeping the king out of danger

There is one more piece of the puzzle. Nothing so far stops your RAJA from stepping onto a square
where it can be captured. The RAJA's moves each carry a **condition**:

```json
{
  "id": 0,
  "step": [1, 0],
  "actions": [
    { "state": "EMPTY", "action": "MOVE" },
    { "state": "ENEMY", "action": "CAPTURE" }
  ],
  "conditions": [{ "condition": "NOT_ATTACKED" }]
}
```

A **condition** is a guard on a move: the move is only offered if the condition holds.
`NOT_ATTACKED` holds when the square the piece would land on is not attacked by any enemy piece.
So the RAJA may step onto a square only if it is safe there.

We will meet many more conditions in the chapters ahead. This is the first one.

## What the engine does

From these few lines the engine can already play Chaturanga's pieces:

- it offers each piece its moves, including slides that stop at the right square,
- it lets pieces capture, and never lets them land on a friend,
- it keeps the RAJA off attacked squares,
- and it ends the game on checkmate or stalemate.

## Try it

```sh
cargo run --bin modern-chess-cli apps/docs/examples/chaturanga_pieces.json
```

Start with the ones that have room to move. Ask for the RATHA at `[0, 0]`:

```text
Select position to see available moves (e.g., [0, 1]): [0, 0]
Valid moves:
[0, 1]: "MOVE"
[0, 2]: "MOVE"
...
[0, 6]: "MOVE"
[0, 7]: "CAPTURE"
```

It can slide all the way up the column, and the last square is a capture of BLACK's RATHA in the
far corner. (The moves come out in no particular order, so yours may be shuffled.) The GAJA at
`[2, 0]` shows `[0, 2]` and `[4, 2]`, and nothing else.

Notice that the RATHA cannot go sideways: the ASHVA is standing right next to it, so the move
is blocked by an ally.

## Now change it

1. **Slow down the rook.** Add `"repeat": { "times": 3 }` in place of `{ "loop": true }` on all
   four RATHA moves. Select the RATHA at `[0, 0]` again and count its moves.
2. **Invent a piece.** Add a CAMEL to `pieces` that leaps three squares one way and one the
   other, as in `[3, 1]` and `[1, 3]`, and so on in all eight directions. Give each player one in
   `starting_positions` (somewhere that is empty). You have just designed a piece from
   [Tamerlane chess](https://en.wikipedia.org/wiki/Tamerlane_chess).
3. **Take the guard off the king.** Delete the `"conditions"` line from every RAJA move. Then
   change `starting_positions` so that an enemy RATHA covers a square next to your RAJA. The RAJA
   now offers to step into the line of fire. This is what `NOT_ATTACKED` was quietly doing for
   you.

In the next chapter we bring back the foot soldiers. They are the first piece with a real
"forward", they promote when they reach the far side of the board, and they teach us how to talk
about *zones* on the board.
