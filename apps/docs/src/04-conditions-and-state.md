# 4. Conditions and memory

Chess looks simple until you meet its special moves. The double pawn push, en passant and
castling all depend on *what has happened so far*: "this pawn has not moved yet", "that pawn just
jumped two squares", "this rook has never moved".

The engine has no special code for any of them. They are built from three ordinary ideas:

1. **Tests** that guard a move.
2. **State**: a note the engine attaches to a piece and remembers.
3. **Effects**: things that happen together with a move, to more than one piece.

The spec for this chapter is `crates/engine/specs/chess.json`.

## Tests: the blocks

The test steps of a move are a set of guards. **All** of them must pass or the move is not
offered, and each one judges the board as the steps before it left it. You have met three tests so
far, `COUNT`, `ATTACKED` and `IN_ZONE`. A test is built from four kinds of block:

- a **place**: a square, named from the move;
- a **region**: one or more places;
- a **filter**: which pieces count;
- the **test** itself, which says what must be true.

### Places

There are two places:

| Place | It names |
|-------|----------|
| `{ "place": "START" }` | the square the moving piece stands on |
| `{ "place": "TARGET" }` | the square the move lands on |

Either can take an `offset`, a `[dx, dy]` step away from it, written the same neutral way as
`step`. The engine turns it around for each player. So `{ "place": "START", "offset": [0, 1] }`
is the square in front of the moving piece.

### Regions

| Region | Written | Its places |
|--------|---------|------------|
| `AT` | `{ "region": "AT", "at": P }` | the one place `P` |
| `PATH` | `{ "region": "PATH", "from": P, "to": P }` | the squares strictly between the two, on a straight line. Add `"ends": ["FROM", "TO"]` to include either end |
| `AROUND` | `{ "region": "AROUND", "of": P }` | the squares next to `P` |
| `BOARD` | `{ "region": "BOARD" }` | every place of the board (with a `where`, as in chapter 2, only those holding a matching piece) |
| `LINE` | `{ "region": "LINE", "through": P, "along": [0, 1] }` | the whole line through `P`, both ways (chapter 7) |

Two more regions come later: `GROUP`, a chain of connected stones (chapter 10), and `FOLLOW`, a walk
along links (chapter 12).

### Filters

A filter, written in `where`, says which pieces count. Every field you write must hold:

| Field | A piece counts when |
|-------|---------------------|
| `"owner": "ALLY"` | it is yours (`"ENEMY"`: an opponent's; `"ANY"`, the default: anyone's) |
| `"codes": ["ROOK"]` | its code is in the list |
| `"moved": false` | it has never moved (`true`: it has moved) |
| `"flag": "EN_PASSANT"` | it carries that note (more below) |

### Tests

| Test | It holds when... |
|------|------------------|
| `COUNT` | the number of pieces in a region that pass the filter is between `min` and `max` |
| `ATTACKED` | an enemy attacks some square of the region |
| `IN_ZONE` | a place is in your part of a zone |
| `NOT` | the test in `of` does not hold |
| `ALL` | every test in `of` holds |
| `ANY` | at least one test in `of` holds |

`COUNT`'s `min` is `1` when you leave it out, and `max` has no limit. So a bare `COUNT` means
*"there is at least one"*. For *"there is none"*, wrap it in `NOT`, or write `"min": 0, "max": 0`.
(Writing only `"max": 0` is refused: `min` would still be `1`.)

## The double push

A pawn may move two squares, but only from its starting square, and only if the square in front
of it is free:

```json
{
  "id": 1,
  "step": [0, 2],
  "do": [
    { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } } } },
    { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START" } }, "where": { "moved": false } },
    { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START", "offset": [0, 1] } } } },
    { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } },
    { "effect": "SET_FLAG", "at": { "place": "TARGET" }, "flag": "EN_PASSANT", "turns": 1 }
  ]
}
```

- **The first test** asks for an empty target, as the one-square step does.
- **The second test** counts the pieces on `START` that have never moved. The pawn itself stands
  there, so this reads: *"this pawn has not moved yet."* The engine counts every piece's moves
  for you.
- **The third test** counts the pieces on the square in front, at `[0, 1]`. `NOT` makes it
  *"nothing stands there"*. That is how the pawn is stopped from jumping over a piece in front of
  it.
- The **`MOVE`** effect moves the pawn. The `SET_FLAG` after it is the memory, below.

Try it: choose a pawn in the starting position and you get two squares.

```text
Selected: 4,1
Places: 4,2 (Move), 4,3 (Move)
```

## Memory: `SET_FLAG` and `flag`

Look at the last step of the double push:

```json
{ "effect": "SET_FLAG", "at": { "place": "TARGET" }, "flag": "EN_PASSANT", "turns": 1 }
```

An **effect** is a change to the board. This one attaches a note, called a **state flag**, to the
piece at `TARGET`, which is the pawn that has just moved (the `MOVE` came first). The note can be
any word you like. Here it is `EN_PASSANT`.

`turns: 1` means the note survives for exactly one more move, the opponent's reply, and then the
engine removes it. `turns` counts single moves by either player, not full rounds. Leave it out
and the note stays on the piece forever.

Another move can look for the note with a filter's `flag`. That is how en passant works. This is
the pawn's capture to the forward left, written as its own move beside the ordinary diagonal
capture (the real move also ends with the promotion step from chapter 3, which is left out here):

```json
{
  "id": 3,
  "step": [-1, 1],
  "do": [
    { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } } } },
    { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START", "offset": [-1, 0] } }, "where": { "flag": "EN_PASSANT" } },
    { "effect": "REMOVE", "at": { "place": "START", "offset": [-1, 0] } },
    { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } }
  ]
}
```

Read the steps in order:

1. **The target square is empty.** An ordinary capture is a different move of the pawn, with the
   same step and its own tests (it asks for an enemy on the target).
2. **The square beside us**, at `[-1, 0]`, holds a piece that carries the `EN_PASSANT` note.
3. **`REMOVE`** takes that piece off the board.
4. **`MOVE`** puts the pawn on the empty square.

If the move is offered and played, the black pawn beside us is gone and ours stands where it
jumped over. Two ideas appear here:

- **Two moves with the same step.** A move that does one thing on an empty square and another on
  an enemy one is written as two moves. Each has its own tests. There is no list of cases inside
  a move: a test says what is allowed, and an effect says what happens.
- **An effect can change a place other than the target.** A normal capture removes the piece on
  the destination, because `MOVE` removes whatever stood there. En passant removes a neighbour
  first, with a `REMOVE` at an offset.

Each removal still counts as a capture: the history marks the move as one, and the material count
shows the pawn taken.

You can play it out in the CLI. After `e2-e4`, `a7-a6`, `e4-e5`, `d7-d5` (in board coordinates:
`[4, 1]`→`[4, 3]`, `[0, 6]`→`[0, 5]`, `[4, 3]`→`[4, 4]`, `[3, 6]`→`[3, 4]`), choose the pawn on
`[4, 4]` and it gets a second move:

```text
Selected: 4,4
Places: 4,5 (Move), 3,5 (Move)
```

Take it, and the black pawn on `[3, 4]` disappears. If you wait one more turn before capturing,
the note has expired and the `[3, 5]` move is gone. That is `turns: 1`.

## Castling

Castling combines everything. Here is the white kingside castle (the king steps two squares):

```json
{
  "id": 9,
  "players": ["WHITE"],
  "step": [2, 0],
  "do": [
    { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } } } },
    { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START" } }, "where": { "moved": false } },
    { "test": "NOT", "of": { "test": "ATTACKED", "in": { "region": "AT", "at": { "place": "START" } } } },
    { "test": "COUNT", "in": { "region": "PATH", "from": { "place": "START" }, "to": { "place": "START", "offset": [3, 0] } }, "min": 0, "max": 0 },
    { "test": "NOT", "of": { "test": "ATTACKED", "in": { "region": "AT", "at": { "place": "TARGET" } } } },
    { "test": "NOT", "of": { "test": "ATTACKED", "in": { "region": "PATH", "from": { "place": "START" }, "to": { "place": "TARGET" }, "ends": ["TO"] } } },
    { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START", "offset": [3, 0] } }, "where": { "owner": "ALLY", "codes": ["ROOK"] } },
    { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START", "offset": [3, 0] } }, "where": { "moved": false } },
    { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } },
    { "effect": "MOVE", "from": { "place": "START", "offset": [3, 0] }, "to": { "place": "START", "offset": [1, 0] } }
  ]
}
```

Read the guards top to bottom, as a checklist the king must pass:

| Guard | In plain words |
|-------|----------------|
| `NOT` `COUNT` at `TARGET` | the landing square is empty |
| `COUNT` at `START`, `moved: false` | the king has never moved |
| `NOT` `ATTACKED` at `START` | the king is not in check right now (the square it stands on is safe) |
| `COUNT` in the `PATH` to `[3, 0]`, `min` and `max` `0` | nothing stands between the king and the rook |
| `NOT` `ATTACKED` at `TARGET` | the landing square is safe |
| `NOT` `ATTACKED` in the `PATH` to `TARGET`, with its `TO` end | every square the king crosses, and the one it lands on, is safe |
| `COUNT` at `[3, 0]`, `ALLY` and `ROOK` | one of your own rooks is there |
| `COUNT` at `[3, 0]`, `moved: false` | that piece has never moved |

Two details are new:

- **A `PATH` to any square.** A `PATH` runs between any two places. Here it runs from the king to
  the rook, so it covers exactly the squares real castling needs empty, including the square
  beside the rook that the king never crosses. The other `PATH` runs from the king to where it
  lands, and `"ends": ["TO"]` adds the landing square to it.
- **`players`.** The king is not in the middle of the back rank, and the board is turned around
  for Black, so the rook is at a different distance for each player. For a step of `[2, 0]`
  White's rook is 3 squares away, while Black's is 4. Instead of one move that tries to serve both,
  the spec has one move **per player and per side**, each restricted with a `players` list. A
  move without `players` is available to everyone, as every move so far has been. Offsets are
  still written from the player's own point of view.

The finale is the **second `MOVE`** effect. It moves *another* piece: the rook, from `[3, 0]` to
`[1, 0]`, in the same turn. The first `MOVE` moves the king, the piece you chose, and counts a
move for it. The second moves the rook, which you did not choose, so the rook still counts as
never having moved. Play it in the CLI, on the position of the board below, and you can watch it:

```text
Selected: 4,0
Places: 5,0 (Move), 3,0 (Move), 4,1 (Move), 5,1 (Move), 3,1 (Move), 2,0 (Move), 6,0 (Move)
```

(The position is `boards/special_moves.json`, which `apps/docs/build.sh` writes.) Choose `[6, 0]`
and the king lands on `g1` while the rook jumps to `f1`. `[2, 0]` castles on the other side.

Because the guards check for the rook and for an empty path, castling is not offered when the rook
has gone, has moved, or has a piece between it and the king.

## What the engine does

- It remembers how many times each piece has moved, so "first move" is always answerable. Only
  the piece you chose counts a move: a rook that castling moved still counts none.
- It keeps the state notes, counts their `turns` down, and removes them.
- It builds the "attacked squares" map from the same moves you wrote. That is how `ATTACKED`
  works.
- It lets one move change several pieces, which is what the effects do.

## Play it here

A small position built for this chapter. White's king and both rooks are still at home, so
**castling** is on offer: click the king and look two squares to either side. The white pawn on `g2`
can use the **double push**. And the white pawn on `e5` waits beside a black pawn on `d7`: push the
`g2` pawn one square, let Black play `d7` to `d5`, then capture it **en passant**. The `e5` pawn lands
on `d6`, an *empty* square, and the black pawn disappears. Every one of these moves comes from the
tests, state and effects you just read.

<modern-chess-board spec="boards/special_moves.json" skin="boards/chess.skin.json">Playable board (needs JavaScript).</modern-chess-board>

## Now change it

1. **A longer memory.** Change the `turns` on the double push from `1` to `3`. The note now
   survives three moves, so a pawn that jumped two squares can be taken en passant on either of
   the opponent's next two turns. (A `2` would not be enough: the note counts moves by both
   players, so it expires before the opponent's second turn.) This is a variant, not chess, and
   it takes one number to make.
2. **No en passant.** Remove the `SET_FLAG` step from the double push. No pawn ever carries the
   `EN_PASSANT` flag, so the tests that look for it never pass, and en passant disappears from
   the game.
3. **Cautious pawns.** Give the pawn's single step (`id` `0`) the king's test,
   `{ "test": "NOT", "of": { "test": "ATTACKED", "in": { "region": "AT", "at": { "place": "TARGET" } } } }`,
   as a step before its `MOVE`. Pawns now refuse to step onto a square an enemy attacks.

Next, we look at the other ways a game can end.
