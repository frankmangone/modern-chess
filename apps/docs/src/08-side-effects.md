# 8. Side effects

A move normally changes one thing: a piece goes from here to there, and maybe removes an enemy.
In chapter 4 you saw moves that did more: castling also moved a rook, and en passant also removed a
pawn. Those extras are called **side effects**.

This chapter shows that side effects can build whole games. We look at two:

- **Atomic Chess**: every capture causes an explosion.
- **Ataxx**: pieces clone themselves and convert their neighbours.

## The toolbox

You have already used three side effects. Here is the full list, with the two new ones at the
bottom:

| Side effect | What it does |
|-------------|--------------|
| `SET_STATE` | Attaches a named note to the moved piece (chapter 4) |
| `CAPTURE` | Removes the piece at a relative square (chapter 4: en passant) |
| `MOVE` | Moves *another* piece from one relative square to another (chapter 4: castling) |
| `EXPLODE` | Removes every piece around the landing square |
| `COPY_SOURCE` | Leaves a copy of the moved piece on the square it started from |
| `CONVERT` | Changes the owner of an enemy piece at a relative square |

### Where a side effect lives

Side effects can be written in two places:

- On the **move**: they fire whenever the move is played.
- On an **action** inside the move: they fire only when *that* action happens. Chapter 4's en
  passant uses this.

The difference matters for Atomic Chess.

## Atomic Chess: `EXPLODE`

**The rule.** Chess, plus one change: when a piece captures, it explodes. The capturing piece, the
captured piece, and every piece on the eight surrounding squares disappear. Pawns are the
exception: they survive an explosion unless they were the piece directly involved.

The spec is `crates/engine/specs/atomic.json`. The piece moves are exactly chess's. The only
difference is that every **capture action** carries a side effect:

```json
{
  "state": "ENEMY",
  "action": "CAPTURE",
  "side_effects": [{ "action": "EXPLODE", "exempt_codes": ["PAWN"] }]
}
```

Because it is attached to the `ENEMY` → `CAPTURE` action, it fires only when the piece captures.
Ordinary moves to empty squares do not explode. (If you attached it to the whole move instead, every
step would blow up.)

| Field | Default | Meaning |
|-------|---------|---------|
| `radius` | `1` | How many squares away the blast reaches |
| `exempt_codes` | `[]` | Piece codes that survive the blast |

Set up a position with a white rook on `[0, 0]`, a black knight on `[0, 4]`, a black bishop on
`[1, 4]`, a black queen on `[1, 3]`, and a black pawn on `[0, 5]`. Play the rook to `[0, 4]`. The
rook, knight, bishop and queen are all gone, but the pawn is untouched. Your own pieces count too:
the capturing rook is destroyed by its own explosion.

## Ataxx: `COPY_SOURCE` and `CONVERT`

**The rule.** Ataxx has a single kind of piece, a STONE, on a 7×7 board. On your turn you pick a
stone and either:

- **clone** it onto an adjacent empty square (the original stays put), or
- **jump** it two squares away (the original leaves).

In both cases, every enemy stone next to the landing square becomes yours.

The spec is `crates/engine/specs/ataxx.json`. It has no leader, and `stalemate_loses` is on, so
a player who cannot move loses. There is only one piece, but its moves use two new side effects.

### A clone move

```json
{
  "id": 0,
  "step": [1, 0],
  "actions": [{ "state": "EMPTY", "action": "MOVE" }],
  "side_effects": [
    { "action": "COPY_SOURCE" },
    { "action": "CONVERT", "target": [0, -1], "piece": "STONE" },
    { "action": "CONVERT", "target": [0,  1], "piece": "STONE" },
    { "action": "CONVERT", "target": [1, -1], "piece": "STONE" },
    { "action": "CONVERT", "target": [1,  1], "piece": "STONE" },
    { "action": "CONVERT", "target": [2, -1], "piece": "STONE" },
    { "action": "CONVERT", "target": [2,  0], "piece": "STONE" },
    { "action": "CONVERT", "target": [2,  1], "piece": "STONE" }
  ]
}
```

Take it apart:

- The base move is an ordinary step onto an empty square: `[1, 0]`.
- **`COPY_SOURCE`** puts a copy of the piece back on the square it just left. A move that
  vacates the source plus a copy that refills it equals a clone. There are no fields to fill in.
- **`CONVERT`** flips one square. If an enemy piece stands at `target`, it becomes the current
  player's. If the square is empty, off the board or already yours, nothing happens.

The `target` of a `CONVERT` is an offset **from where the piece started**, like all the offsets in
side effects. The landing square is `[1, 0]` away, so the eight squares around it, minus the
source itself, are `[0,±1]`, `[1,±1]`, `[2,±1]` and `[2,0]`: seven entries. One side effect per
square is the way to say "all the neighbours". The spec has no shortcut for it.

### A jump move

A jump is a step of length two. It has the same `CONVERT` side effects as a clone, but **no**
`COPY_SOURCE`:

```json
{
  "id": 8,
  "step": [2, 0],
  "actions": [{ "state": "EMPTY", "action": "MOVE" }],
  "side_effects": [
    { "action": "CONVERT", "target": [1, -1], "piece": "STONE" },
    { "action": "CONVERT", "target": [1,  0], "piece": "STONE" },
    { "action": "CONVERT", "target": [1,  1], "piece": "STONE" },
    { "action": "CONVERT", "target": [2, -1], "piece": "STONE" },
    { "action": "CONVERT", "target": [2,  1], "piece": "STONE" },
    { "action": "CONVERT", "target": [3, -1], "piece": "STONE" },
    { "action": "CONVERT", "target": [3,  0], "piece": "STONE" },
    { "action": "CONVERT", "target": [3,  1], "piece": "STONE" }
  ]
}
```

With no `COPY_SOURCE`, the source square is left empty and the stone has jumped. A jump has eight
`CONVERT` entries where a clone has seven, because the source square is no longer next to the
landing square, so all eight squares around the landing can hold an enemy.

A stone has eight clone moves (the adjacent squares) and sixteen jump moves (the squares at
distance two), so the file is long but regular. The engine does not mind: each one is an entry
in `moves`.

### Playing it

```sh
cargo run --bin modern-chess-cli crates/engine/specs/ataxx.json
```

Make a scratch copy with one RED stone on `[0, 0]` and two BLUE stones on `[2, 0]` and `[1, 2]`.
Select the red stone and you are offered `[0, 1]`, `[1, 0]` and `[1, 1]` (clones), and `[0, 2]`,
`[2, 1]` and `[2, 2]` (jumps). The squares with blue stones on them are not offered, because
the stone's actions only allow empty squares. Clone to `[1, 0]` and the blue stone on `[2, 0]`
turns red, while your stone on `[0, 0]` stays where it was. Jump to `[0, 2]` instead, and the
source square is left empty, and the blue stone on `[1, 2]`, right next to the landing square,
turns red.

## What the engine does

- It applies all of a move's side effects together with the move itself, so the rest of the turn
  sees the new board. Converted stones really are yours when the engine decides what to do next.
- It lets any number of side effects ride on one move or one action.

## Play it here

**Atomic Chess**, set up exactly as in the example above. Play the white rook up to the black knight
and watch the explosion: the rook, knight, bishop and queen all disappear, and the black pawn stays.

<modern-chess-board spec="boards/atomic_blast.json" skin="boards/chess.skin.json">Playable board (needs JavaScript).</modern-chess-board>

**Ataxx**, the whole spec. Click a stone, then clone it one square away or jump it two. Clone next
to an enemy stone, and the enemy stones touching your new stone change colour.

<modern-chess-board spec="boards/ataxx.json" skin="boards/ataxx.skin.json">Playable board (needs JavaScript).</modern-chess-board>

## Now change it

1. **Pawns burn too.** In `atomic.json`, change `"exempt_codes": ["PAWN"]` to `[]` on one
   capture action. Pawns near that capture now die.
2. **A bigger bang.** Set `"radius": 2` on the `EXPLODE` in each of the ROOK's capture actions.
   The blast now reaches two squares out. Use the position from earlier plus a black knight on
   `[2, 4]`: the rook capturing on `[0, 4]` now destroys that knight too, but still not the pawn
   on `[0, 5]`.
3. **Jumps that do not convert.** Delete the `side_effects` of the jump move with
   `"step": [0, 2]`. Re-run the scratch game above and jump to `[0, 2]`: the blue stone on
   `[1, 2]` now stays blue. You have invented a variant in which only clones capture.
4. **Cloning for chess.** Give a chess piece of your choice a `COPY_SOURCE` side effect on its
   ordinary move. What happens when a rook clones itself every time it moves?

Next: the last big rule family. Games in which you *must* capture.
