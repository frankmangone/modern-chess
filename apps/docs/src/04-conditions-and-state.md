# 4. Conditions and memory

Chess looks simple until you meet its special moves. The double pawn push, en passant and
castling all depend on *what has happened so far*: "this pawn has not moved yet", "that pawn just
jumped two squares", "this rook has never moved".

The engine has no special code for any of them. They are built from three ordinary ideas:

1. **Conditions** that guard a move.
2. **State**: a note the engine attaches to a piece and remembers.
3. **Side effects**: extra things that happen together with a move.

The spec for this chapter is `crates/engine/specs/chess.json`.

## Tests: the blocks

A move's `conditions` list is a set of guards. **All** of them must pass or the move is not
offered. Each guard is a **test**. You have met two, `ATTACKED` and `IN_ZONE`. A test is built from
four kinds of block:

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
| `LINE` | `{ "region": "LINE", "through": P, "along": [0, 1] }` | the whole line through `P`, both ways (chapter 7) |

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
  "actions": [{ "state": "EMPTY", "action": "MOVE" }],
  "conditions": [
    { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START" } }, "where": { "moved": false } },
    { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START", "offset": [0, 1] } } } }
  ],
  "side_effects": [{ "action": "SET_STATE", "state": "EN_PASSANT", "duration": 1 }]
}
```

- **The first test** counts the pieces on `START` that have never moved. The pawn itself stands
  there, so this reads: *"this pawn has not moved yet."* The engine counts every piece's moves
  for you.
- **The second test** counts the pieces on the square in front, at `[0, 1]`. `NOT` makes it
  *"nothing stands there"*. That is how the pawn is stopped from jumping over a piece in front of
  it.

Try it: select a pawn in the starting position and you get two squares.

```text
Select position to see available moves (e.g., [0, 1]): [4, 1]
Valid moves:
[4, 2]: "MOVE"
[4, 3]: "MOVE"
```

## Memory: `SET_STATE` and `flag`

Look at the last line of the double push:

```json
"side_effects": [{ "action": "SET_STATE", "state": "EN_PASSANT", "duration": 1 }]
```

A **side effect** is something extra that happens when a move is made. This one attaches a
note, called a **state flag**, to the pawn that just moved. The note can be any word you like.
Here it is `EN_PASSANT`.

`duration: 1` means the note survives for exactly one more move, the opponent's reply, and then
the engine removes it. `duration` counts single moves by either player, not full rounds. Leave
it out and the note stays on the piece forever.

Another move can look for the note with a filter's `flag`. That is how en passant works:

```json
{
  "id": 2,
  "step": [-1, 1],
  "actions": [
    { "state": "ENEMY", "action": "CAPTURE" },
    {
      "state": "EMPTY",
      "action": "MOVE",
      "conditions": [
        { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START", "offset": [-1, 0] } }, "where": { "flag": "EN_PASSANT" } }
      ],
      "side_effects": [{ "action": "CAPTURE", "target": [-1, 0] }]
    }
  ]
}
```

This is the pawn's diagonal-left step. Read the `actions` as two cases:

1. **An enemy is on the target square.** An ordinary capture.
2. **The target square is empty.** Only allowed if the square **beside** us, at `[-1, 0]`, holds
   a piece that carries the `EN_PASSANT` note. When it does, we move there *and* a side effect removes the pawn at
   `[-1, 0]`.

Two new ideas appear here:

- **Conditions and side effects can live on an individual action**, not just on the move. The
  condition above only guards the `EMPTY` case. The ordinary capture is unaffected.
- The side effect `CAPTURE` with a `target` removes a piece that is **not** on the square we
  land on. A normal capture removes the piece on the destination. En passant removes a
  neighbour.

You can play it out in the CLI. After `e2-e4`, `a7-a6`, `e4-e5`, `d7-d5` (in board coordinates:
`[4, 1]`→`[4, 3]`, `[0, 6]`→`[0, 5]`, `[4, 3]`→`[4, 4]`, `[3, 6]`→`[3, 4]`), select the pawn on
`[4, 4]` and it gets a second move:

```text
Select position to see available moves (e.g., [0, 1]): [4, 4]
Valid moves:
[4, 5]: "MOVE"
[3, 5]: "MOVE"
```

Take it, and the black pawn on `[3, 4]` disappears. If you wait one more turn before capturing,
the note has expired and the `[3, 5]` move is gone. That is `duration: 1`.

## Castling

Castling combines everything. Here is the white kingside castle (the king steps two squares):

```json
{
  "id": 9,
  "players": ["WHITE"],
  "step": [2, 0],
  "actions": [{ "state": "EMPTY", "action": "MOVE" }],
  "conditions": [
    { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START" } }, "where": { "moved": false } },
    { "test": "NOT", "of": { "test": "ATTACKED", "in": { "region": "AT", "at": { "place": "START" } } } },
    { "test": "COUNT", "in": { "region": "PATH", "from": { "place": "START" }, "to": { "place": "START", "offset": [3, 0] } }, "min": 0, "max": 0 },
    { "test": "NOT", "of": { "test": "ATTACKED", "in": { "region": "AT", "at": { "place": "TARGET" } } } },
    { "test": "NOT", "of": { "test": "ATTACKED", "in": { "region": "PATH", "from": { "place": "START" }, "to": { "place": "TARGET" }, "ends": ["TO"] } } },
    { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START", "offset": [3, 0] } }, "where": { "owner": "ALLY", "codes": ["ROOK"] } },
    { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START", "offset": [3, 0] } }, "where": { "moved": false } }
  ],
  "side_effects": [
    { "action": "MOVE", "piece": "ROOK", "from": [3, 0], "to": [1, 0] }
  ]
}
```

Read the guards top to bottom, as a checklist the king must pass:

| Guard | In plain words |
|-------|----------------|
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

The finale is the **side effect** `MOVE`. It moves *another* piece: the rook, from `[3, 0]` to
`[1, 0]`, in the same turn. Play it in the CLI and you can watch it:

```text
Select position to see available moves (e.g., [0, 1]): [4, 0]
Valid moves:
[4, 1]: "MOVE"
[5, 0]: "MOVE"
[6, 0]: "MOVE"
```

Choose `[6, 0]` and the king lands on `g1` while the rook jumps to `f1`.

Because the guards check for the rook and for an empty path, castling is not offered when the rook
has gone, has moved, or has a piece between it and the king.

## What the engine does

- It remembers how many times each piece has moved, so "first move" is always answerable.
- It keeps the state notes, counts their duration down, and removes them.
- It builds the "attacked squares" map from the same moves you wrote. That is how `ATTACKED`
  works.
- It lets one move change several pieces, which is what a side effect does.

## Play it here

A small position built for this chapter. White's king and both rooks are still at home, so
**castling** is on offer: click the king and look two squares to either side. The white pawn on `g2`
can use the **double push**. And the white pawn on `e5` waits beside a black pawn on `d7`: push the
`g2` pawn one square, let Black play `d7` to `d5`, then capture it **en passant**. The `e5` pawn lands
on `d6`, an *empty* square, and the black pawn disappears. Every one of these moves comes from the
conditions, state and side effects you just read.

<modern-chess-board spec="boards/special_moves.json" skin="boards/chess.skin.json">Playable board (needs JavaScript).</modern-chess-board>

## Now change it

1. **A longer memory.** Change the `duration` on the double push from `1` to `3`. The note now
   survives three moves, so a pawn that jumped two squares can be taken en passant on either of
   the opponent's next two turns. (A `2` would not be enough: the note counts moves by both
   players, so it expires before the opponent's second turn.) This is a variant, not chess, and
   it takes one number to make.
2. **No en passant.** Remove the `SET_STATE` line from the double push. No pawn ever carries the
   `EN_PASSANT` flag, so the tests that look for it never pass, and en passant disappears from
   the game.
3. **Cautious pawns.** Give the pawn's single step (`id` `0`) the king's test,
   `{ "test": "NOT", "of": { "test": "ATTACKED", "in": { "region": "AT", "at": { "place": "TARGET" } } } }`,
   in a `conditions` list. Pawns now refuse to step onto a square an enemy attacks.

Next, we look at the other ways a game can end.
