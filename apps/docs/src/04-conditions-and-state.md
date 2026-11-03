# 4. Conditions and memory

Chess looks simple until you meet its special moves. The double pawn push, en passant and
castling all depend on *what has happened so far*: "this pawn has not moved yet", "that pawn just
jumped two squares", "this rook has never moved".

The engine has no special code for any of them. They are built from three ordinary ideas:

1. **Conditions** that guard a move.
2. **State**: a note the engine attaches to a piece and remembers.
3. **Side effects**: extra things that happen together with a move.

The spec for this chapter is `crates/engine/specs/chess.json`.

## Conditions you already know, and new ones

A move's `conditions` list is a set of guards. **All** of them must pass or the move is not
offered. You have met `NOT_ATTACKED` and zones such as `REACH_END`. This chapter adds a few more:

| Condition | It passes when... |
|-----------|-------------------|
| `PIECE_FIRST_MOVE` | the piece at a given square has never moved |
| `DEPENDS_ON` | another move of the same piece is available |
| `PATH_PIECE_COUNT` | the number of pieces *between* the start and the end is within limits |
| `PATH_NOT_ATTACKED` | no square along the way is attacked |
| `CHECK_STATE` | the piece at a given square carries a named note |

Several of them take a `position`: a `[dx, dy]` offset **from the moving piece**, written the
same neutral way as `step`. The engine turns it around for each player.

## The double push

A pawn may move two squares, but only from its starting square, and only if the square in front
of it is free:

```json
{
  "id": 1,
  "step": [0, 2],
  "actions": [{ "state": "EMPTY", "action": "MOVE" }],
  "conditions": [
    { "condition": "PIECE_FIRST_MOVE", "position": [0, 0] },
    { "condition": "DEPENDS_ON", "move_id": 0 }
  ],
  "side_effects": [{ "action": "SET_STATE", "state": "EN_PASSANT", "duration": 1 }]
}
```

- **`PIECE_FIRST_MOVE` at `[0, 0]`.** An offset of `[0, 0]` points at the moving piece itself, so
  this reads: *"this pawn has not moved yet."* The engine counts every piece's moves for you.
- **`DEPENDS_ON`.** Move `0` is the pawn's ordinary one-square step. This condition says the
  double push is only available when the single step is. That is how the pawn is stopped from
  jumping over a piece in front of it. This is the job of the `id` field you have been writing
  since chapter 1.

Try it: select a pawn in the starting position and you get two squares.

```text
Select position to see available moves (e.g., [0, 1]): [4, 1]
Valid moves:
[4, 2]: "MOVE"
[4, 3]: "MOVE"
```

## Memory: `SET_STATE` and `CHECK_STATE`

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

Another move can look for the note with `CHECK_STATE`. That is how en passant works:

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
        { "condition": "CHECK_STATE", "state": "EN_PASSANT", "position": [-1, 0] }
      ],
      "side_effects": [{ "action": "CAPTURE", "target": [-1, 0] }]
    }
  ]
}
```

This is the pawn's diagonal-left step. Read the `actions` as two cases:

1. **An enemy is on the target square.** An ordinary capture.
2. **The target square is empty.** Only allowed if the piece **beside** us, at `[-1, 0]`, carries
   the `EN_PASSANT` note. When it does, we move there *and* a side effect removes the pawn at
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
  "step": [2, 0],
  "actions": [{ "state": "EMPTY", "action": "MOVE" }],
  "conditions": [
    { "condition": "PIECE_FIRST_MOVE", "position": [0, 0] },
    { "condition": "NOT_ATTACKED", "position": [0, 0] },
    { "condition": "PATH_PIECE_COUNT", "max": 0 },
    { "condition": "NOT_ATTACKED" },
    { "condition": "PATH_NOT_ATTACKED" },
    { "condition": "PIECE_FIRST_MOVE", "position": [3, 0], "absent_ok": true },
    { "condition": "PIECE_FIRST_MOVE", "position": [4, 0], "absent_ok": true }
  ],
  "side_effects": [
    { "action": "MOVE", "piece": "ROOK", "from": [3, 0], "to": [1, 0] },
    { "action": "MOVE", "piece": "ROOK", "from": [4, 0], "to": [1, 0] }
  ]
}
```

Read the guards top to bottom, as a checklist the king must pass:

| Guard | In plain words |
|-------|----------------|
| `PIECE_FIRST_MOVE` at `[0, 0]` | the king has never moved |
| `NOT_ATTACKED` at `[0, 0]` | the king is not in check right now (the square it stands on is safe) |
| `PATH_PIECE_COUNT` max `0` | no pieces between the king and where it lands |
| `NOT_ATTACKED` | the landing square is safe |
| `PATH_NOT_ATTACKED` | every square the king crosses is safe |
| `PIECE_FIRST_MOVE` at `[3, 0]` / `[4, 0]` | the rook has never moved |

The rook check appears **twice**, at two different offsets, with `"absent_ok": true`. The reason
is that a `step` and its offsets are written once and reused by both players, and the two players
keep their rooks in different places relative to the king. Whichever offset has nothing to check
simply passes. `absent_ok` means: *"if there is no piece on that square, do not complain."*

The finale is the **side effect** `MOVE`. It moves *another* piece: the rook, from `[3, 0]` to
`[1, 0]`, in the same turn. Like the guards, it appears twice for the same reason, and only the
one that finds a rook does anything. Play it in the CLI and you can watch it:

```text
Select position to see available moves (e.g., [0, 1]): [4, 0]
Valid moves:
[4, 1]: "MOVE"
[5, 0]: "MOVE"
[6, 0]: "MOVE"
```

Choose `[6, 0]` and the king lands on `g1` while the rook jumps to `f1`.

> **Careful.** The castling in `chess.json` is faithful to the common cases but not to every
> corner of the rules. For example, queenside castling does not check the square next to the
> rook, and a king can castle if its rook was captured and nothing stands on the corner. When you
> write your own special moves, test the corners. See [Known limitations](known-limitations.md).

## What the engine does

- It remembers how many times each piece has moved, so "first move" is always answerable.
- It keeps the state notes, counts their duration down, and removes them.
- It builds the "attacked squares" map from the same moves you wrote. That is how
  `NOT_ATTACKED` and `PATH_NOT_ATTACKED` work.
- It lets one move change several pieces, which is what a side effect does.

## Now change it

1. **A longer memory.** Change the `duration` on the double push from `1` to `3`. The note now
   survives three moves, so a pawn that jumped two squares can be taken en passant on either of
   the opponent's next two turns. (A `2` would not be enough: the note counts moves by both
   players, so it expires before the opponent's second turn.) This is a variant, not chess, and
   it takes one number to make.
2. **No en passant.** Remove the `SET_STATE` line from the double push. The `CHECK_STATE` moves
   can never fire, so en passant disappears from the game.
3. **Cautious pawns.** Add `{ "condition": "NOT_ATTACKED" }` to the pawn's single step (`id`
   `0`). Pawns now refuse to step onto a square an enemy attacks.

Next, we look at the other ways a game can end.
