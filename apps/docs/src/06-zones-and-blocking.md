# 6. Zones and blocking

Xiangqi (Chinese chess) is a rich test of the spec. Its board has a river in the middle, its
generals are locked in palaces, its horses can be blocked, and its cannons capture in a way no
other piece does. Almost all of it is described with tools you already own. We add three new ones:
**restrictions**, **explicit blockers**, and **`until`**.

The spec is `crates/engine/specs/xiangqi.json`. The board is 9 columns by 10 rows, and the two
players are called RED and BLACK.

## The rules we are building

- The **GENERAL** and the **ADVISOR** never leave their 3×3 *palace*.
- The **ELEPHANT** leaps two squares diagonally, can be blocked, and never crosses the river.
- The **HORSE** moves like a knight but can be blocked by a piece on its first step.
- The **CHARIOT** is a rook.
- The **CANNON** slides like a rook to move, but must *jump over exactly one piece* to capture.
- The **SOLDIER** steps forward. After it crosses the river it may also step sideways.
- The two generals may never face each other on an open file.

## Restricting a piece to a zone

A palace is a zone, defined exactly like `REACH_END` in chapter 3, with one list per player:

```json
{
  "code": "PALACE",
  "type": "POSITION",
  "check": {
    "RED":   [[3,0],[4,0],[5,0],[3,1],[4,1],[5,1],[3,2],[4,2],[5,2]],
    "BLACK": [[3,7],[4,7],[5,7],[3,8],[4,8],[5,8],[3,9],[4,9],[5,9]]
  }
}
```

To lock a piece into the zone, give the piece a `restrictions` list. This sits on the piece, next
to `moves`, and applies to **all** its moves:

```json
{ "code": "GENERAL", "restrictions": [{ "zone": "PALACE" }], "moves": [ ... ] }
```

Any move that would land outside the zone is quietly dropped. It is a shortcut: without it, you
would have to add the zone as a condition to every single move. In the CLI, a RED general on
`[4, 0]` is offered exactly `[3, 0]`, `[5, 0]` and `[4, 1]`: palace squares only.

The ELEPHANT uses the same tool with a different zone, `OWN_SIDE` (your half of the board), which
is how it is kept from crossing the river. An elephant on `[2, 4]`, right at the edge of its half,
may go back to `[0, 2]` or `[4, 2]` and never forward to `[4, 6]`.

> A restriction filters where a piece may *land*. It does not change what a piece *threatens*.
> A general locked in its palace still attacks along the whole file. We use that in a moment.

## Blocking a leap: `LEG_EMPTY`

A knight leaps over anything. A Xiangqi horse does not. It first steps one square straight, and
that square (the "leg") must be empty before the diagonal finish. The condition for this is
`LEG_EMPTY`, which checks one named square:

```json
{
  "id": 4,
  "step": [1, 2],
  "conditions": [{ "condition": "LEG_EMPTY", "position": [0, 1] }],
  "actions": [
    { "state": "EMPTY", "action": "MOVE" },
    { "state": "ENEMY", "action": "CAPTURE" }
  ]
}
```

The step `[1, 2]` is a horse leap: one across, two forward. Its leg is the square straight ahead,
`[0, 1]`. If any piece stands there, the move is not offered. A horse needs eight moves, each
with its own leg:

| Leap | Leg |
|------|-----|
| `[ 1,  2]`, `[-1,  2]` | `[0,  1]` |
| `[ 1, -2]`, `[-1, -2]` | `[0, -1]` |
| `[ 2,  1]`, `[ 2, -1]` | `[ 1, 0]` |
| `[-2,  1]`, `[-2, -1]` | `[-1, 0]` |

Put a horse on `[4, 4]` with a friendly soldier on `[4, 5]` and both forward leaps, to `[3, 6]`
and `[5, 6]`, disappear. The other six remain.

The ELEPHANT's "blocked eye" is similar, but its path runs along the leap itself, so it uses
`PATH_PIECE_COUNT` (chapter 4) with a `max` of `0`: nothing may stand on the square in the
middle of the diagonal.

## The cannon: `until`

The cannon has two behaviours, so it gets two groups of moves. In chapter 2 you saw that
`repeat` makes a piece slide. The slide stops at the first piece it meets. A new field,
`until`, lets you say *which* kind of square stops it.

**Moving.** Slide over empty squares, never capture:

```json
{
  "id": 0,
  "step": [1, 0],
  "repeat": { "until": "NOT_EMPTY", "loop": true },
  "actions": [{ "state": "EMPTY", "action": "MOVE" }]
}
```

`"NOT_EMPTY"` is the default you were using all along: stop at the first piece.

**Capturing.** Keep sliding past anything, and capture an enemy, but only if exactly one piece
(the *screen*) lies between the cannon and the target:

```json
{
  "id": 4,
  "step": [1, 0],
  "conditions": [{ "condition": "PATH_PIECE_COUNT", "min": 1, "max": 1 }],
  "repeat": { "until": "ENEMY", "loop": true },
  "actions": [{ "state": "ENEMY", "action": "CAPTURE" }]
}
```

- `"until": "ENEMY"` makes the slide continue through empty squares and friends, and stop at an
  enemy.
- `PATH_PIECE_COUNT` with `min: 1, max: 1` counts the pieces strictly between the cannon and the
  square it is looking at. It must be exactly one.

> **A limitation to know about.** Because the slide stops at the *first enemy*, a cannon can
> use a **friendly** piece as the screen, but not an enemy one. A cannon on `[0, 0]` with a
> friendly soldier on `[0, 3]` and an enemy chariot on `[0, 8]` is offered `[0, 8]` as a capture.
> With an *enemy* piece on `[0, 3]` instead, it is not. In real Xiangqi either colour may be the
> screen. See [Known limitations](known-limitations.md).

## The soldier and the river

The soldier moves straight ahead. After crossing the river it may also step sideways. That
needs a zone for "the river or beyond", and a condition on the sideways moves:

```json
{ "id": 1, "step": [1, 0], "conditions": [{ "condition": "RIVER_OR_BEYOND" }],
  "actions": [{ "state": "EMPTY", "action": "MOVE" }, { "state": "ENEMY", "action": "CAPTURE" }] }
```

A zone named in a move's `conditions` is tested against the square the move **lands on**. A
sideways step lands on the same row, so the condition really asks, *"is this soldier already
across?"*. A soldier on `[2, 3]` has only its forward step. One on `[6, 5]` also gets `[5, 5]` and
`[7, 5]`.

## The flying general: `threat_only`

The last rule: the two generals may not face each other. Each GENERAL gets one extra move that
it can **never play**. It exists only to count as a *threat*:

```json
{
  "id": 4,
  "step": [0, 1],
  "threat_only": true,
  "repeat": { "loop": true },
  "actions": [{ "state": "ENEMY", "action": "CAPTURE" }]
}
```

`threat_only: true` means the move is used when the engine works out which squares each side
attacks, but is never offered to the player. This one attacks straight up the file, so when the
two generals face each other with nothing between them, each one is "in check" from the other.
Because the general's palace restriction only filters where it *lands*, the threat still
reaches across the whole board.

This is the way to describe "a piece that threatens something it cannot actually do". It is
the same trick, with no special engine support, that gives Xiangqi its flying general.

> **Another limitation.** The engine uses these threats when it asks "is the game over?". It
> does not currently stop a player from *making* a move that leaves their own general facing the
> other: a general may step onto an open file and the move is accepted. The same is true in
> chess of pinned pieces. See [Known limitations](known-limitations.md).

## What the engine does

- It limits pieces to zones, and filters where they may land.
- It reads a blocker square (`LEG_EMPTY`) and counts pieces along a path (`PATH_PIECE_COUNT`).
- It slides until a chosen kind of square (`until`).
- It tracks threats separately from moves, so you can threaten without being able to move.

## Try it

```sh
cargo run --bin modern-chess-cli crates/engine/specs/xiangqi.json
```

The board is 9 wide and 10 tall. Select the horse at `[1, 0]`. It is offered only `[0, 2]` and
`[2, 2]`. Its leap towards `[3, 1]` is missing because the elephant on `[2, 0]` stands on that
leap's leg. Now select the elephant itself: it can reach `[0, 2]` and `[4, 2]`.

## Now change it

1. **A bigger palace.** Add `[2,1]` and `[6,1]` to RED's `PALACE` list. Red's general may now step
   sideways out of the 3×3.
2. **Free the horse.** Delete the `LEG_EMPTY` condition from one horse move and you have a
   hybrid knight that is unblockable in that one direction.
3. **A third piece in the zone.** Add `"restrictions": [{ "zone": "PALACE" }]` to the CHARIOT.
   A chariot trapped in a palace plays quite differently.

Next, a game where captured pieces change sides: Shogi.
