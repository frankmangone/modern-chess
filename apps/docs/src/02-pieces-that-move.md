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

In chapter 1, a marcher could go only to an empty square. A piece that captures asks for less:

```json
"do": [
  { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } }, "where": { "owner": "ALLY" } } },
  { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } }
]
```

Read it as: *"If no ally of mine stands on the target, move there."* The `where` is a **filter**:
it says which pieces the `COUNT` counts, here only the mover's own (`"owner": "ALLY"`). An empty
square passes, and so does a square held by an enemy. The `MOVE` effect puts the piece on the
target, and whatever stood there is removed: that is a **capture**. A piece can never land on a
square held by a friend, because the test fails there.

Almost every piece in Chaturanga uses exactly this `do`.

## Leapers: one step, one move

The ASHVA moves like a chess knight. It has eight possible landing squares, and each one is a
separate move:

```json
{ "id": 0, "step": [ 2,  1], "do": [ ...move or capture... ] },
{ "id": 1, "step": [ 2, -1], "do": [ ...move or capture... ] },
{ "id": 2, "step": [-2,  1], "do": [ ...move or capture... ] },
{ "id": 3, "step": [-2, -1], "do": [ ...move or capture... ] },
{ "id": 4, "step": [ 1,  2], "do": [ ...move or capture... ] },
{ "id": 5, "step": [ 1, -2], "do": [ ...move or capture... ] },
{ "id": 6, "step": [-1,  2], "do": [ ...move or capture... ] },
{ "id": 7, "step": [-1, -2], "do": [ ...move or capture... ] }
```

(In the real file every `do` list is written out in full. We shortened it here.)

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
  "repeat": { "loop": true },
  "do": [
    { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } }, "where": { "owner": "ALLY" } } },
    { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } }
  ]
}
```

This reads: *"Step up. If you can, step up again, and again, as far as you can go."* The engine
offers every square along the way where the `do` allows a move. It stops when:

- it reaches the **edge** of the board,
- it reaches an **ally**. That square is not offered (the test fails), and neither is anything
  beyond it,
- it reaches an **enemy**. That square *is* offered, as a capture, but nothing beyond it is.

(A slide stops at the first piece it meets, whoever's it is. Chapter 6 shows how to choose
another stopping place.)

The RATHA has four of these, one for each straight direction (`[1, 0]`, `[-1, 0]`, `[0, 1]`
and `[0, -1]`).

You can also give a slide a limit. `"repeat": { "times": 3 }` goes at most three squares:

```json
"repeat": { "times": 3 }
```

A RATHA with that limit, standing at `[0, 0]` with a clear column, can reach `[0, 1]`, `[0, 2]`
and `[0, 3]`.

## Protecting your RAJA

So far a piece is only a piece. A game like chess adds one that matters more than the rest: you
may never leave your king where the enemy could take it. That rule is not about one piece or one
move, it is about *every* move. So it is written once, at the top of the spec, in a list called
`every_action`:

```json
"every_action": [
  { "do": [
    { "test": "NOT", "of": { "test": "REMOVED", "where": { "owner": "ALLY", "codes": ["RAJA"] } } },
    { "test": "ANY", "of": [
      { "test": "NOT", "of": { "test": "ATTACKED",
          "in": { "region": "BOARD", "where": { "owner": "ALLY", "codes": ["RAJA"] } }, "by": "ENEMY" } },
      { "test": "REMOVED", "where": { "owner": "ENEMY", "codes": ["RAJA"] } }
    ] }
  ] }
]
```

An entry of `every_action` holds steps, like a move's `do`, and the engine runs them after every
turn's own steps, as if you had written them at the end of each. They look at the board *after*
the move, and when one of them fails, that move is not offered. Read as sentences, these two say:

- *"This move did not take my own RAJA off the board."*
- *"After this move, no enemy attacks a square holding my RAJA, unless the move took the enemy's
  RAJA."* `BOARD` with a `where` is every square that holds a piece passing the filter, here
  your RAJA, wherever it stands.

(You will read tests closely in a moment, and `REMOVED` again in chapter 9. For now the sentences
are enough.) From this one entry the engine understands:

- **Check.** Your RAJA is *in check* when an enemy piece could capture it on its next move. The
  engine works that out from the very same `moves` you already wrote. There is no separate list
  of "attack rules".
- **Safety.** No move that leaves your RAJA attacked is offered. A piece that shields its RAJA
  cannot step away from the line of attack (it is *pinned*), and when you are in check, only the
  moves that end the check are offered.

One more line, also at the top of the spec, tells the engine how the game *ends*:

```json
"leader": ["RAJA"]
```

The `leader` list names the piece (or pieces) the end of the game looks at, when the player to
move has no move left:

- **Checkmate.** You have no move, **and** your leader is attacked: you lose.
- **Stalemate.** You have no move, but your leader is *not* attacked: the game is a draw.

This is the end-of-game rule from chapter 1, made smarter. Without a `leader`, a player with no
moves just draws the game. With one, the engine can tell the difference between "stuck" and
"checkmated". `leader` does nothing else: it is the `every_action` entry that keeps the RAJA safe.

### A second test

The RAJA's moves in the spec each carry two tests before the `MOVE`:

```json
{
  "id": 0,
  "step": [1, 0],
  "do": [
    { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } }, "where": { "owner": "ALLY" } } },
    { "test": "NOT", "of": { "test": "ATTACKED", "in": { "region": "AT", "at": { "place": "TARGET" } } } },
    { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } }
  ]
}
```

The first you know. The second is a guard on the move: the move is only offered if the test holds.
Every test step works this way, and a move may have as many as it needs. Read the second one from
the inside out:

- `{ "place": "TARGET" }` is the square the piece would land on.
- `{ "region": "AT", ... }` is just that one square.
- `ATTACKED` holds when an enemy piece attacks a square in the region.
- `NOT` turns it around.

Together: *"the landing square is not attacked"*.

For the RAJA this guard is, strictly speaking, redundant: the `every_action` entry already
refuses any move of any piece that leaves it attacked. We keep the test because it is the
simplest one to read, and because `ATTACKED` is useful on its own for pieces that are *not* the
RAJA, and for rules like castling (chapter 4).

We will meet the other blocks in the chapters ahead. This is the first test you have read block
by block.

## What the engine does

From these few lines the engine can already play Chaturanga's pieces:

- it offers each piece its moves, including slides that stop at the right square,
- it lets pieces capture, and never lets them land on a friend,
- it keeps the RAJA off attacked squares,
- and it ends the game on checkmate or stalemate.

## Play it here

Click any piece and look at the highlighted squares. The ASHVA leaps over its neighbours; the
RATHA slides until something is in the way; every capture ends on the enemy's square. All of
this comes from the spec you just read.

<modern-chess-board spec="boards/chaturanga_pieces.json" skin="boards/chaturanga.skin.json">Playable board (needs JavaScript).</modern-chess-board>

## Try it

```sh
cargo run --bin modern-chess-cli apps/docs/examples/chaturanga_pieces.json
```

Start with the ones that have room to move. Choose the RATHA at `[0, 0]`:

```text
Places: 0,0 (Piece), 1,0 (Piece), 2,0 (Piece), 3,0 (Piece), 4,0 (Piece), 5,0 (Piece), 6,0 (Piece), 7,0 (Piece)
Answer (a place, a code, a hand piece, an action, or cancel): 0,0
Selected: 0,0
Places: 0,1 (Move), 0,2 (Move), 0,3 (Move), 0,4 (Move), 0,5 (Move), 0,6 (Move), 0,7 (Move)
```

It can slide all the way up the column, and the last square is a capture of BLACK's RATHA in the
far corner. (The CLI does not say which places capture: they are all `Move`.) The GAJA at `[2, 0]`
shows `[0, 2]` and `[4, 2]`, and nothing else. Type `cancel` between tries.

Notice that the RATHA cannot go sideways: the ASHVA is standing right next to it, so the move
is blocked by an ally.

## Now change it

1. **Slow down the rook.** Add `"repeat": { "times": 3 }` in place of `{ "loop": true }` on all
   four RATHA moves. Choose the RATHA at `[0, 0]` again and count its moves.
2. **Invent a piece.** Add a CAMEL to `pieces` that leaps three squares one way and one the
   other, as in `[3, 1]` and `[1, 3]`, and so on in all eight directions. Give each player one in
   `starting_positions` (somewhere that is empty). You have just designed a piece from
   [Tamerlane chess](https://en.wikipedia.org/wiki/Tamerlane_chess).
3. **Take the guard off the king.** Delete the `ATTACKED` test step from every RAJA move. Then
   change `starting_positions` so that an enemy RATHA covers a square next to your RAJA. The RAJA
   is still not offered that square, because the `every_action` entry protects it. Now also
   delete `every_action`: with nothing to protect it, the RAJA offers to step into the line of
   fire.

In the next chapter we bring back the foot soldiers. They are the first piece with a real
"forward", they promote when they reach the far side of the board, and they teach us how to talk
about *zones* on the board.
