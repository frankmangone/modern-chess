# 6. Zones and blocking

Xiangqi (Chinese chess) is a rich test of the spec. Its board has a river in the middle, its
generals are locked in palaces, its horses can be blocked, and its cannons capture in a way no
other piece does. Almost all of it is described with tools you already own: zones and tests. We
use them in three new ways: to **keep a piece in a zone**, to **block a leap**, and to choose
where a slide stops with **`until`**.

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

## Keeping a piece in a zone

A palace is a zone, declared under `zones` exactly like `REACH_END` in chapter 3, with one list
per player:

```json
{
  "name": "PALACE",
  "places": {
    "RED":   [[3,0],[4,0],[5,0],[3,1],[4,1],[5,1],[3,2],[4,2],[5,2]],
    "BLACK": [[3,7],[4,7],[5,7],[3,8],[4,8],[5,8],[3,9],[4,9],[5,9]]
  }
}
```

To keep a piece in the zone, give **each** of its moves an `IN_ZONE` test on the square it lands
on:

```json
{
  "id": 0,
  "step": [1, 0],
  "conditions": [{ "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "PALACE" }],
  "actions": [
    { "state": "EMPTY", "action": "MOVE" },
    { "state": "ENEMY", "action": "CAPTURE" }
  ]
}
```

Any move that would land outside the zone is not offered. In the CLI, a RED general on `[4, 0]`
is offered exactly `[3, 0]`, `[5, 0]` and `[4, 1]`: palace squares only.

The ELEPHANT uses the same test with a different zone, `OWN_SIDE` (your half of the board), which
is how it is kept from crossing the river. An elephant on `[2, 4]`, right at the edge of its half,
may go back to `[0, 2]` or `[4, 2]` and never forward to `[4, 6]`.

> A test decides which moves are *offered*. It does not change what a piece *threatens*.
> A general kept in its palace still attacks along the whole file. We use that in a moment.

## Blocking a leap

A knight leaps over anything. A Xiangqi horse does not. It first steps one square straight, and
that square (the "leg") must be empty before the diagonal finish. A `COUNT` on that one square,
turned around by `NOT`, says so:

```json
{
  "id": 4,
  "step": [1, 2],
  "conditions": [
    { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START", "offset": [0, 1] } } } }
  ],
  "actions": [
    { "state": "EMPTY", "action": "MOVE" },
    { "state": "ENEMY", "action": "CAPTURE" }
  ]
}
```

The step `[1, 2]` is a horse leap: one across, two forward. Its leg is the square straight ahead,
`START` moved by `[0, 1]`. If any piece stands there, the move is not offered. A horse needs
eight moves, each with its own leg:

| Leap | Leg |
|------|-----|
| `[ 1,  2]`, `[-1,  2]` | `[0,  1]` |
| `[ 1, -2]`, `[-1, -2]` | `[0, -1]` |
| `[ 2,  1]`, `[ 2, -1]` | `[ 1, 0]` |
| `[-2,  1]`, `[-2, -1]` | `[-1, 0]` |

Put a horse on `[4, 4]` with a friendly soldier on `[4, 5]` and both forward leaps, to `[3, 6]`
and `[5, 6]`, disappear. The other six remain.

One more detail. A region whose place is off the board **cannot be formed**, and a `COUNT` over
it is false. `NOT` flips that as usual, so a `NOT` of a `COUNT` on a square off the board holds:
nothing stands on a square that does not exist.

The ELEPHANT's "blocked eye" is similar, but its middle square lies on the leap itself, so it uses
a `PATH` from `START` to `TARGET` (chapter 4) with `"min": 0, "max": 0`: nothing may stand on the
square in the middle of the diagonal.

## The cannon: `until`

The cannon has two behaviours, so it gets two groups of moves. In chapter 2 you saw that
`repeat` makes a piece slide. The slide stops at the first piece it meets. A new field,
`until`, lets you say *which* square stops it. It is a test, judged on each square the slide
reaches, with `TARGET` that square. When it holds, the slide goes no further.

**Moving.** Slide over empty squares, never capture:

```json
{
  "id": 0,
  "step": [1, 0],
  "repeat": { "until": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } } }, "loop": true },
  "actions": [{ "state": "EMPTY", "action": "MOVE" }]
}
```

This `until` is the default you were using all along: stop at the first square with a piece on
it. The spec leaves it out and writes just `"repeat": { "loop": true }`.

**Capturing.** Keep sliding past *everything*, and capture an enemy, but only if exactly one
piece (the *screen*, of either colour) lies between the cannon and the target:

```json
{
  "id": 4,
  "step": [1, 0],
  "conditions": [
    { "test": "COUNT", "in": { "region": "PATH", "from": { "place": "START" }, "to": { "place": "TARGET" } }, "min": 1, "max": 1 }
  ],
  "repeat": { "until": { "test": "ANY", "of": [] }, "loop": true },
  "actions": [{ "state": "ENEMY", "action": "CAPTURE" }]
}
```

- `ANY` holds when at least one of its tests does. With no tests at all it never holds, so this
  slide never stops early: it runs to the edge of the board, past empty squares, friends and
  enemies alike.
- The `COUNT` over the `PATH` from the cannon to the square it is looking at counts the pieces
  strictly between them. It must be exactly one. That is what does the real work here: a piece
  with nothing in front of it, or with two pieces in front of it, is out of reach.

Be careful with an `until` that stops at enemies,
`{ "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } }, "where": { "owner": "ENEMY" } }`.
It stops the slide at the *first* enemy piece, so an enemy piece standing in front of the target
would end the slide before the target is reached, and a cannon could only ever use a friendly
screen.

## The soldier and the river

The soldier moves straight ahead. After crossing the river it may also step sideways. That
needs a zone for "the river or beyond", and a test on the sideways moves:

```json
{ "id": 1, "step": [1, 0], "conditions": [{ "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "RIVER_OR_BEYOND" }],
  "actions": [{ "state": "EMPTY", "action": "MOVE" }, { "state": "ENEMY", "action": "CAPTURE" }] }
```

The test asks about `TARGET`, the square the move **lands on**. A sideways step lands on the same
row, so it really asks, *"is this soldier already across?"*. You could ask that directly, with
`{ "place": "START" }`; for a sideways step the answer is the same. A soldier on `[2, 3]` has only its forward step. One on `[6, 5]` also gets `[5, 5]` and
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
Because the general's `IN_ZONE` tests only decide which moves are offered, the threat still
reaches across the whole board.

This is the way to describe "a piece that threatens something it cannot actually do". It is
the same trick, with no special engine support, that gives Xiangqi its flying general.

The engine uses these threats whenever it asks whether a general is attacked. That includes
discarding moves that would leave your own general attacked, so a general may not step onto an
open file facing the other.

## What the engine does

- It keeps pieces in zones, with an `IN_ZONE` test on each move.
- It counts the pieces on a blocker square (`AT`) and along a path (`PATH`).
- It slides until a test holds (`until`).
- It tracks threats separately from moves, so you can threaten without being able to move.

## Try it

```sh
cargo run --bin modern-chess-cli crates/engine/specs/xiangqi.json
```

The board is 9 wide and 10 tall. Select the horse at `[1, 0]`. It is offered only `[0, 2]` and
`[2, 2]`. Its leap towards `[3, 1]` is missing because the elephant on `[2, 0]` stands on that
leap's leg. Now select the elephant itself: it can reach `[0, 2]` and `[4, 2]`.

## Now change it

1. **A bigger palace.** Add `[2,1]` and `[6,1]` to RED's `PALACE` places. Red's general may now step
   sideways out of the 3×3.
2. **Free the horse.** Delete the leg test from one horse move and you have a hybrid knight
   that is unblockable in that one direction.
3. **A third piece in the zone.** Add the `PALACE` test to each of the CHARIOT's moves. A
   chariot trapped in a palace plays quite differently.

Next, a game where captured pieces change sides: Shogi.
