# Game Spec Format

A game is fully described by a single JSON file. The engine reads it, validates it, and runs the
game without any hardcoded rules. This document explains every field.

---

## Top-level structure

```json
{
  "name": "CHESS",
  "leader": "KING",
  "board": { ... },
  "players": [ ... ],
  "turns": { ... },
  "conditions": [ ... ],
  "pieces": [ ... ]
}
```

| Field | Required | Description |
|-------|----------|-------------|
| `name` | yes | Human-readable game name. |
| `leader` | no | Piece code whose capture ends the game (e.g. `"KING"`). Omit for games without checkmate. |
| `board` | yes | Board geometry. |
| `players` | yes | One entry per player with direction and starting layout. |
| `turns` | yes | Turn order. |
| `conditions` | no | Global named conditions referenced by move definitions. |
| `pieces` | yes | All piece types and their move rules. |

---

## `board`

```json
"board": {
  "dimensions": [8, 8],
  "disabled_positions": [[3, 3], [4, 4]]
}
```

| Field | Required | Description |
|-------|----------|-------------|
| `dimensions` | yes | `[cols, rows]` — both must be ≥ 1. Positions are 0-indexed: `[0,0]` is the bottom-left. |
| `disabled_positions` | no | Squares that don't exist (think of hollow boards or irregular shapes). Pieces cannot be placed on or moved to disabled positions. |

---

## `players`

Each player entry declares their name, movement orientation, and where their pieces start.

```json
"players": [
  {
    "name": "WHITE",
    "direction": [[1, 0], [0, 1]],
    "starting_positions": [
      { "piece": "PAWN", "positions": [[0,1],[1,1],[2,1],[3,1],[4,1],[5,1],[6,1],[7,1]] },
      { "piece": "ROOK", "positions": [[0,0],[7,0]] },
      { "piece": "KING", "positions": [[4,0]] }
    ]
  }
]
```

### `direction` — the orientation matrix

Every move `step` written in piece specs is defined from the perspective of a "neutral" player
moving in the +y direction. At build time the engine multiplies each step by this 2×2 matrix to
produce the actual step for that player.

Must be a valid rotation (or rotation+reflection): `det(matrix) = ±1`.

| Player orientation | Matrix |
|--------------------|--------|
| Moves up (+y) | `[[1,0],[0,1]]` — identity |
| Moves down (−y) | `[[-1,0],[0,-1]]` — 180° rotation |
| Moves right (+x) | `[[0,-1],[1,0]]` — 90° counter-clockwise |
| Moves left (−x) | `[[0,1],[-1,0]]` — 90° clockwise |

So in a standard two-player game, WHITE gets the identity and BLACK gets 180°. In a four-player
game you assign 90° rotations to the side players.

### `starting_positions`

A list of `{ "piece": "<CODE>", "positions": [[x,y], ...] }` entries. Every `piece` value must
match a code defined in the `pieces` array.

---

## `turns`

```json
"turns": {
  "order": ["WHITE", "BLACK"],
  "start_at": 0
}
```

| Field | Required | Description |
|-------|----------|-------------|
| `order` | yes | Cycle of player names. Can include the same name multiple times for unequal turn ratios. |
| `start_at` | no | Index into `order` where the game starts. Default `0`. |

---

## `conditions` — global named conditions

These are board-aware predicates that moves can reference by name. Currently two types are
supported.

### `POSITION` type

True when the piece's **destination** square is in the listed set for the current player.

```json
{
  "code": "REACH_END",
  "type": "POSITION",
  "check": {
    "WHITE": [[0,7],[1,7],[2,7],[3,7],[4,7],[5,7],[6,7],[7,7]],
    "BLACK": [[0,0],[1,0],[2,0],[3,0],[4,0],[5,0],[6,0],[7,0]]
  }
}
```

Each player name maps to a list of `[x, y]` positions. The condition is true when the landing
square matches any position in the current player's list.

Use this for promotion zones, scoring squares, or any destination-restricted move.

### `STATE` type

*(Defined in the spec layer but used internally — see `CHECK_STATE` in move conditions below.)*

---

## `pieces`

The complete catalogue of piece types. Every piece that can ever appear on the board (including
promoted forms) must be listed here.

```json
"pieces": [
  {
    "code": "PAWN",
    "name": "pawn",
    "moves": [ ... ]
  }
]
```

| Field | Required | Description |
|-------|----------|-------------|
| `code` | yes | Unique identifier used everywhere else (starting positions, promotions, etc.). |
| `name` | no | Human-readable label. Not used by the engine. |
| `moves` | yes | List of move definitions (see below). |

The CLI renders the first three characters of `code` inside each board cell, so keep codes
descriptive enough that the three-character prefix is unambiguous.

---

## Move definitions

Each entry in `moves` describes one move pattern for a piece.

```json
{
  "id": 0,
  "step": [0, 1],
  "actions": [ ... ],
  "conditions": [ ... ],
  "modifiers": [ ... ],
  "side_effects": [ ... ],
  "repeat": { ... }
}
```

| Field | Required | Description |
|-------|----------|-------------|
| `id` | yes | Integer unique within this piece. Used by the `DEPENDS_ON` condition. |
| `step` | yes | `[dx, dy]` offset applied to the piece's position. Written from the neutral player's perspective; the engine rotates it per-player using the direction matrix. |
| `actions` | yes | What happens depending on the square's occupancy (see below). |
| `conditions` | no | Move-level guards — all must pass or the move is not offered. |
| `modifiers` | no | Post-move transformations (e.g. promotion). |
| `side_effects` | no | Move-level side effects that always fire (use action-level side_effects for conditional ones). |
| `repeat` | no | Makes the step repeat (for sliding pieces like rooks and bishops). |

### `step` and direction

Steps are always written from the perspective of a player moving in the +y direction. The engine
transforms them at build time. Example: a forward step `[0, 1]` becomes `[0, -1]` for a player
with a 180° direction matrix (moving down the board).

---

## `actions`

Determines what the piece can do when the target square has a given occupancy. At least one action
is required per move.

```json
"actions": [
  { "state": "EMPTY",  "action": "MOVE" },
  { "state": "ENEMY",  "action": "CAPTURE" }
]
```

### `state` — target square occupancy

| Value | Meaning |
|-------|---------|
| `"EMPTY"` | No piece on the target square. |
| `"ENEMY"` | A piece belonging to a different player. |
| `"ALLY"` | A piece belonging to the same player. |

Only the action whose `state` matches the actual occupancy fires. You can have multiple actions
for the same move with different states.

### `action`

| Value | Effect |
|-------|--------|
| `"MOVE"` | Move the piece to the target square. |
| `"CAPTURE"` | Remove the occupant and move the piece there. |

### Action-level `conditions`

An action can have its own condition list. These are checked *after* the move-level conditions and
only for that specific action. Useful for conditional captures (en passant) where the same step
can be a normal move or a special capture depending on board state.

```json
{
  "state": "EMPTY",
  "action": "MOVE",
  "conditions": [
    { "condition": "CHECK_STATE", "state": "EN_PASSANT", "position": [-1, 0] }
  ],
  "side_effects": [ ... ]
}
```

### Action-level `side_effects`

Side effects attached to a specific action only fire when that action is taken. See the
[side effects](#side-effects) section.

---

## Move-level `conditions`

All conditions in this list must pass for the move to be offered at all.

```json
"conditions": [
  { "condition": "FIRST_MOVE" },
  { "condition": "PATH_EMPTY" },
  { "condition": "CHECK_STATE", "state": "EN_PASSANT", "position": [-1, 0] }
]
```

### Available conditions

#### `FIRST_MOVE`
The moving piece has never moved (`total_moves == 0`).

No extra fields.

```json
{ "condition": "FIRST_MOVE" }
```

---

#### `DEPENDS_ON`
Another move on this piece (identified by `move_id`) must have at least one legal landing square
in the current position. Useful for two-square pawn jumps that should only be available when
the one-square move is also legal.

```json
{ "condition": "DEPENDS_ON", "move_id": 0 }
```

| Extra field | Description |
|-------------|-------------|
| `move_id` | The `id` of another move in the same piece's move list. |

---

#### `PIECE_FIRST_MOVE`
The piece at a relative position has never moved. Returns false if the position is off-board or
empty.

```json
{ "condition": "PIECE_FIRST_MOVE", "position": [3, 0] }
```

| Extra field | Description |
|-------------|-------------|
| `position` | `[dx, dy]` offset from the moving piece's source, written in neutral coordinates and rotated per-player at build time. |

---

#### `ROOK_FIRST_MOVE`
Like `PIECE_FIRST_MOVE` but returns true (passes vacuously) when the position is off-board or
empty. Designed for castling specs where two conditions cover both possible rook positions — one
will always be vacuous for any given player.

```json
{ "condition": "ROOK_FIRST_MOVE", "position": [3, 0] }
```

---

#### `CHECK_STATE`
The piece at a relative position carries a named state flag.

```json
{ "condition": "CHECK_STATE", "state": "EN_PASSANT", "position": [-1, 0] }
```

| Extra field | Description |
|-------------|-------------|
| `state` | The flag name to look for. |
| `position` | Relative offset to the piece that must carry the flag. |

---

#### `PATH_EMPTY`
Every square strictly between the source and the target (derived from `step`) is empty. For a
step of magnitude 1 this is always true. For a knight jump `[1, 2]` it is also always true (the
path check uses the step's unit vector, so intermediate squares along the axis are checked, but
the knight's step has no intermediate axis squares).

No extra fields.

```json
{ "condition": "PATH_EMPTY" }
```

---

#### `NOT_ATTACKED`
The target square (source + step) is not in any opponent's attack set.

No extra fields.

```json
{ "condition": "NOT_ATTACKED" }
```

---

#### `PATH_NOT_ATTACKED`
Every square from source up to and including the target is not in any opponent's attack set.
Used for castling's king path safety check.

No extra fields.

```json
{ "condition": "PATH_NOT_ATTACKED" }
```

---

#### Named global conditions (e.g. `REACH_END`)
Any code defined in the top-level `conditions` array can be referenced here by name. Currently
only `POSITION`-type conditions are supported as move conditions.

```json
{ "condition": "REACH_END" }
```

---

## `repeat`

Makes the move step until it hits something or a limit is reached. This is how sliding pieces
(rooks, bishops, queens) are modelled.

```json
"repeat": {
  "until": "NOT_EMPTY",
  "loop": true
}
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `until` | no | `"NOT_EMPTY"` | Stop condition checked on the *next* position before stepping into it. `"NOT_EMPTY"` stops before any piece. The current position's action (CAPTURE if enemy) is still evaluated. |
| `loop` | no | `false` | If true, repeat indefinitely until `until` triggers or the edge of the board. |
| `times` | no | `1` | Maximum repetitions when `loop` is false. |

**Typical patterns:**

```json
// Slides until blocked (rook, bishop, queen)
"repeat": { "loop": true }

// Exactly two steps (pawn double-push)
"repeat": { "times": 2 }
```

---

## `modifiers`

Post-move hooks that can change the move's outcome. The only supported modifier action is
`TRANSFORM` (promotion).

```json
"modifiers": [
  {
    "action": "TRANSFORM",
    "conditions": [{ "condition": "REACH_END" }],
    "options": ["QUEEN", "ROOK", "BISHOP", "KNIGHT"]
  }
]
```

| Field | Required | Description |
|-------|----------|-------------|
| `action` | yes | Must be `"TRANSFORM"`. |
| `conditions` | no | All must pass for the modifier to fire. These use the same condition vocabulary as move-level conditions. |
| `options` | yes | Piece codes the player can choose from. Including the moving piece's own code makes promotion optional (the player can choose to stay). |

When a modifier fires the game enters the `Transforming` phase. The player must call the
`Transform` transition with one of the listed codes before the turn advances.

---

## `side_effects`

Side effects fire alongside a move and produce additional board changes. They can live at the
move level (always fire if the move is legal) or inside an action (fire only when that action is
taken).

### `SET_STATE`
Attaches a named state flag to the moved piece.

```json
{ "action": "SET_STATE", "state": "EN_PASSANT", "duration": 1 }
```

| Field | Description |
|-------|-------------|
| `state` | Flag name. Can be any string. |
| `duration` | Optional countdown in turns. `1` means the flag is visible for one opponent turn, then removed. Omit for a permanent flag. |

State flags are inspected by `CHECK_STATE` conditions. They're the mechanism for tracking
transient facts: en passant eligibility, first-move bonuses, etc.

---

### `CAPTURE`
Removes the piece at a relative position. Used for en passant, where the captured pawn is not on
the target square.

```json
{ "action": "CAPTURE", "target": [-1, 0] }
```

| Field | Description |
|-------|-------------|
| `target` | `[dx, dy]` relative to the moving piece's **source** position, in neutral coordinates (rotated per-player at build time). |

---

### `MOVE`
Moves another piece from one relative position to another. Used for castling.

```json
{ "action": "MOVE", "piece": "ROOK", "from": [3, 0], "to": [1, 0] }
```

| Field | Description |
|-------|-------------|
| `piece` | Optional piece code filter — only moves the piece if its code matches. |
| `from` | Source offset relative to the moving piece's source, in neutral coordinates. |
| `to` | Destination offset relative to the moving piece's source, in neutral coordinates. |

---

## Coordinate conventions

- All `[dx, dy]` values in `step`, condition `position`, and side effect `from`/`to`/`target` are
  written in **neutral coordinates** (as if the player moves in the +y direction).
- The engine rotates them per-player using the direction matrix at build time.
- Absolute positions in `starting_positions` and in the global `conditions.check` lists are
  written in **board coordinates** (origin at bottom-left).

---

## Complete annotated example — en passant

```json
{
  "id": 2,
  "step": [-1, 1],
  "actions": [
    {
      "state": "ENEMY",
      "action": "CAPTURE"
    },
    {
      "state": "EMPTY",
      "action": "MOVE",
      "conditions": [
        { "condition": "CHECK_STATE", "state": "EN_PASSANT", "position": [-1, 0] }
      ],
      "side_effects": [
        { "action": "CAPTURE", "target": [-1, 0] }
      ]
    }
  ]
}
```

Walk-through:

1. The pawn tries to step diagonally forward-left `[-1, 1]`.
2. If the target square has an **enemy** piece, normal diagonal capture.
3. If the target square is **empty**, check whether the piece at `[-1, 0]` (the adjacent square)
   carries the `EN_PASSANT` flag.
4. If yes: move to the empty square *and* remove the pawn at `[-1, 0]` as a side effect.
5. If no: the action is blocked and the move is not offered.

The `EN_PASSANT` flag is set by a separate pawn move's `SET_STATE` side effect when a pawn
advances two squares.

---

## Complete annotated example — kingside castling

```json
{
  "id": 9,
  "step": [2, 0],
  "actions": [{ "state": "EMPTY", "action": "MOVE" }],
  "conditions": [
    { "condition": "FIRST_MOVE" },
    { "condition": "PATH_EMPTY" },
    { "condition": "NOT_ATTACKED" },
    { "condition": "PATH_NOT_ATTACKED" },
    { "condition": "ROOK_FIRST_MOVE", "position": [3, 0] },
    { "condition": "ROOK_FIRST_MOVE", "position": [4, 0] }
  ],
  "side_effects": [
    { "action": "MOVE", "piece": "ROOK", "from": [3, 0], "to": [1, 0] },
    { "action": "MOVE", "piece": "ROOK", "from": [4, 0], "to": [1, 0] }
  ]
}
```

Walk-through:

1. The king steps two squares right `[2, 0]` to an empty square.
2. Guards: king must not have moved; squares between must be empty; destination must not be
   attacked; path must not be attacked.
3. Two `ROOK_FIRST_MOVE` checks at `[3, 0]` and `[4, 0]` — one covers each player's rook
   position (the other is vacuously true for that player because nothing is there).
4. The side effect teleports the matching rook next to the king.

---

## Known limitations

- **No piece drops.** Captured pieces are removed from the game; they cannot be held in hand and
  replayed (this rules out Shogi drops as a built-in mechanic).
- **Promotion is always mandatory.** When a `TRANSFORM` modifier fires the player must choose an
  option; there is no "skip" path. To make promotion *optional*, include the piece's own code in
  the `options` list — choosing it replaces the piece with a fresh copy of itself.
- **`POSITION` conditions check the destination only.** A piece moving out of a special zone does
  not trigger a condition targeting that zone.
- **No per-player `PATH_EMPTY` offsets.** `PATH_EMPTY` derives the unit path from the move's
  `step`; it does not support checking an independent set of squares.
