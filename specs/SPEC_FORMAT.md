# Game Spec Format

A game is fully described by a single JSON file. The engine reads it, validates it, and runs the
game without any hardcoded rules. This document explains every field.

---

## Top-level structure

```json
{
  "name": "CHESS",
  "leader": ["KING"],
  "stalemate_loses": false,
  "hand_enabled": false,
  "allow_pass": false,
  "forbid_repeated_positions": false,
  "liberty_capture": null,
  "draw_conditions": { ... },
  "win_conditions": [ ... ],
  "board": { ... },
  "players": [ ... ],
  "turns": { ... },
  "conditions": [ ... ],
  "pieces": [ ... ]
}
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `name` | yes | — | Human-readable game name. |
| `leader` | no | `[]` | Array of piece codes whose capture triggers game-over detection (e.g. `["KING"]`). Empty array disables checkmate/stalemate detection entirely. |
| `stalemate_loses` | no | `false` | When `true`, a player with no legal moves loses unconditionally (Shogi rule). When `false` (default), no legal moves without check is a draw. |
| `hand_enabled` | no | `false` | When `true`, captured pieces enter the capturing player's hand and can be dropped back onto the board. When `false`, captures permanently remove pieces. |
| `allow_pass` | no | `false` | When `true`, the player to move may give up their turn. See [Passing](#passing). |
| `forbid_repeated_positions` | no | `false` | When `true`, a move may not leave the position as it was after any earlier turn (the *ko* rule). See [Repeated positions](#repeated-positions). |
| `liberty_capture` | no | — | Captures groups of pieces that have no empty point beside them, as in Go. See [Liberty capture](#liberty-capture). |
| `no_opponent_check` | no | `false` | When `true`, every generated move is post-filtered to exclude moves that put any opponent's leader in check. Used for Racing Kings, where giving check is forbidden. |
| `draw_conditions` | no | — | Optional draw rules (repetition, fifty-move, insufficient material). Omit to disable all draw detection. |
| `win_conditions` | no | `[]` | Optional win rules checked after every move. Any entry that fires ends the game, in favour of the moving player (`AREA_SCORE` instead names whoever scores most). |
| `board` | yes | — | Board geometry. |
| `players` | yes | — | One entry per player with direction and starting layout. |
| `turns` | yes | — | Turn order. |
| `conditions` | no | `[]` | Global named conditions referenced by move definitions. |
| `pieces` | yes | — | All piece types and their move rules. |

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

## `draw_conditions`

Optional top-level object configuring automatic draw detection. All fields default to disabled;
omit any you don't need.

```json
"draw_conditions": {
  "repetition_count": 3,
  "fifty_move_halfmoves": 100,
  "fifty_move_pawn_codes": ["PAWN"],
  "insufficient_material": [
    ["KING"],
    ["KING", "BISHOP"],
    ["KING", "KNIGHT"]
  ]
}
```

| Field | Default | Description |
|-------|---------|-------------|
| `repetition_count` | disabled | Declare a draw when the same position (board state + active player + piece state flags) has been reached this many times. Chess uses `3`; Shogi sennichite uses `4`. To forbid the repeating move instead of drawing, see [`forbid_repeated_positions`](#repeated-positions). |
| `fifty_move_halfmoves` | disabled | Declare a draw after this many consecutive half-moves (plies) with no pawn push and no capture. Chess uses `100` (50 full moves). |
| `fifty_move_pawn_codes` | `[]` | Piece codes whose non-capture moves reset the fifty-move counter. Typically `["PAWN"]`. Only meaningful when `fifty_move_halfmoves` is set. |
| `insufficient_material` | `[]` | List of piece-code multisets that represent insufficient mating material. The game is drawn when **every** player's remaining pieces match one of these multisets. Example: `["KING", "BISHOP"]` means king + bishop cannot force checkmate. |

---

## `win_conditions`

Optional array of win rules evaluated (in order) after every move. The first entry to fire ends
the game immediately — the moving player wins, except for `AREA_SCORE`, which ends the game
after a run of passes and gives it to whoever scores most.

```json
"win_conditions": [
  { "type": "PIECE_IN_ZONE", "piece": "KING", "zone": "CENTER" },
  { "type": "OPPONENT_BARE", "exempt": ["KING"] },
  { "type": "CHECK_COUNT", "threshold": 3 },
  { "type": "AREA_SCORE", "pieces": ["STONE"], "territory": true, "bonus": { "WHITE": 7.5 } }
]
```

### `PIECE_IN_ZONE`

The moving player wins when any of their pieces with the given code land in the named zone
(a global `POSITION` condition).

```json
{ "type": "PIECE_IN_ZONE", "piece": "KING", "zone": "CENTER" }
```

| Field | Required | Description |
|-------|----------|-------------|
| `piece` | yes | Code of the piece that must reach the zone (e.g. `"KING"`). |
| `zone` | yes | Name of a global `POSITION`-type condition that defines the winning squares for each player. |

The `zone` condition must be declared in the top-level `conditions` array. Because it is a
`POSITION` condition it can specify different target squares per player, so "reach your opponent's
back rank" is straightforward.

---

### `OPPONENT_BARE`

The moving player wins when every remaining opponent has only exempt pieces on the board. Used for
"bare king" rules in Shatranj and some Shogi-family variants.

```json
{ "type": "OPPONENT_BARE", "exempt": ["KING"] }
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `exempt` | no | `[]` | Piece codes that are ignored when testing for "bare". Win fires when all non-exempt opponent pieces have been captured. |

With `"exempt": ["KING"]` the game ends when an opponent has nothing left but their king. With an
empty `exempt` list the game ends the moment an opponent has no pieces at all.

---

### `CHECK_COUNT`

The moving player wins after putting the opponent's leader in check a cumulative total of
`threshold` times across the whole game. Used for Three-check Chess.

```json
{ "type": "CHECK_COUNT", "threshold": 3 }
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `threshold` | no | `3` | Number of checks required to win. |

The engine keeps a per-player check counter in `GameState.check_counts` (serialised, so it
survives save/restore). Each time a move leaves the opponent's leader in check the counter
increments; when it reaches `threshold` the game ends.

---

### `AREA_SCORE`

Ends the game once enough players have **passed** in a row, and gives it to the player with the
highest score. Equal best scores are a draw. It needs `"allow_pass": true`, since passing is the
only way to reach it. Used for Go (area scoring) and works for Reversi-style counting too.

```json
{
  "type": "AREA_SCORE",
  "pieces": ["STONE"],
  "territory": true,
  "bonus": { "WHITE": 7.5 },
  "after_passes": 2
}
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `pieces` | no | `[]` | Piece codes that count towards a score. Empty means every piece counts. |
| `territory` | no | `false` | When `true`, an empty region also counts for a player if the counted pieces around it all belong to that player. A region touching nobody, or touching more than one player, counts for no one. Only for two-dimensional boards. |
| `bonus` | no | `{}` | Points added to a player's score, by player name (Go's *komi*). Decimals are allowed, so `7.5` can never tie. |
| `after_passes` | no | one pass by every player | How many passes in a row end the game. |

A player's score is one point for each counted piece they have on the board, plus the size of
every empty region they own (with `territory`), plus their bonus. Pieces that are not counted
are ignored: they are not part of a region and do not make one touch a player.

The engine reports the score as it stands at any moment (as if the game ended now) in the
session view's `score`.

**There is no step for agreeing which pieces are dead.** The count is taken on the board as it
is, so a piece that is dead has to be captured in play before the players pass.

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

Each player name maps to a list of `[x, y]` positions in **board coordinates** (absolute, not
relative). The condition is true when the landing square matches any position in the current
player's list.

Use this for promotion zones, scoring squares, or any destination-restricted move.

**Symmetric (absolute) variant**: when all players share an identical position list, the
condition acts as an absolute board-coordinate restriction regardless of orientation. This is
useful for drop restrictions that apply to both players equally — for example, blocking pawn
drops on both terminal ranks of the board:

```json
{
  "code": "TERMINAL_RANK",
  "type": "POSITION",
  "check": {
    "WHITE": [[0,0],[1,0],[2,0],[3,0],[4,0],[5,0],[6,0],[7,0],[0,7],[1,7],[2,7],[3,7],[4,7],[5,7],[6,7],[7,7]],
    "BLACK": [[0,0],[1,0],[2,0],[3,0],[4,0],[5,0],[6,0],[7,0],[0,7],[1,7],[2,7],[3,7],[4,7],[5,7],[6,7],[7,7]]
  }
}
```

Both players get the same set; neither can drop on rows 0 or 7 regardless of their direction
of travel. Contrast this with `REACH_END` above, where the lists differ because "the last rank"
is board-relative for each player.

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
    "moves": [ ... ],
    "demotes_to": null,
    "drop_restrictions": [ ... ]
  }
]
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `code` | yes | — | Unique identifier used everywhere else (starting positions, promotions, etc.). |
| `name` | no | — | Human-readable label. Not used by the engine. |
| `moves` | yes | — | List of move definitions (see below). |
| `restrictions` | no | `[]` | Zone restrictions applied to all moves of this piece. Any destination that fails every restriction is removed from legal moves. See [zone restrictions](#zone-restrictions) below. |
| `demotes_to` | no | `null` | When `hand_enabled` is true and this piece is captured, the piece code that enters the capturer's hand. `null` means the piece enters the hand as itself. Set this on promoted pieces to make them revert to their base form on capture — e.g. `"demotes_to": "PAWN"` on a Shogi TOKIN or a Crazyhouse `P_QUEEN`. See the hand section for the full demotion pattern. |
| `unlimited_supply` | no | `false` | When `true`, any player may place this piece on an empty square at any time, as often as they like, without it ever being in a hand. See [Placing from a supply](#placing-from-a-supply). |
| `drop_restrictions` | no | `[]` | Conditions that **block** a drop on a candidate square. Uses the same condition vocabulary as move conditions. If any restriction fires, the square is excluded from legal drop squares. See `ALLY_ON_FILE` and the hand section below. |
| `value` | no | — | What the piece is worth in the material count shown beside a match. Pieces with no value count as 0, and a game in which no piece has a value shows no material at all. |

The CLI renders the first three characters of `code` inside each board cell, so keep codes
descriptive enough that the three-character prefix is unambiguous.

### Zone restrictions

Restrict all moves of a piece to a named zone. Any destination square not in the zone for the
current player is silently removed from the legal move list.

```json
"restrictions": [
  { "zone": "PALACE" }
]
```

| Field | Required | Description |
|-------|----------|-------------|
| `zone` | yes | Code of a global `POSITION`-type condition. The piece can only land on squares that pass this condition for the current player. |

Multiple restrictions are intersected — the destination must pass **all** of them.

Zone restrictions apply to **legal move generation** only. They do not affect the piece's threat
map (`calculate_threats`), so a GENERAL confined to its palace still attacks squares outside the
palace for purposes like the flying-general detection.

**Xiangqi example** — GENERAL and ADVISOR are confined to their 3×3 palace:

```json
{ "code": "GENERAL", "restrictions": [{ "zone": "PALACE" }], "moves": [ ... ] }
```

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
| `threat_only` | no | When `true`, this blueprint contributes to the piece's **attack map** but never produces legal moves. Default `false`. See [threat_only](#threat_only) below. |
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

### `threat_only`

When `threat_only: true`, the blueprint is included in `calculate_threats` (the attack map used
for check detection) but **never** produces a legal move for the player.

This lets a piece project a threat through a move pattern it would never actually execute. The
canonical use case is Xiangqi's **flying-general rule**: the two generals must never face each
other on the same file with no piece between them. Rather than adding a game-specific flag, the
GENERAL gets one `threat_only` sliding blueprint in the forward direction:

```json
{
  "id": 4,
  "step": [0, 1],
  "threat_only": true,
  "repeat": { "loop": true },
  "actions": [{ "state": "ENEMY", "action": "CAPTURE" }]
}
```

With direction matrices:
- RED's GENERAL (identity direction) projects this threat **up** the file toward BLACK's half.
- BLACK's GENERAL (180° direction) projects it **down** the file toward RED's half.

When the two generals are on the same file with nothing between them, `leader_in_check` returns
`true` because the threat-only blueprint reaches the opposing leader. The current player's
`any_legal_moves` then correctly filters out moves that would expose this flying-general check.

**Important**: `threat_only` interacts with zone restrictions differently from normal moves.
Because zone restrictions only apply to legal move generation (not threats), a GENERAL confined
to its palace still projects the `threat_only` sliding blueprint across the entire file.

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
  { "condition": "PIECE_FIRST_MOVE", "position": [0, 0] },
  { "condition": "PATH_PIECE_COUNT", "max": 0 },
  { "condition": "CHECK_STATE", "state": "EN_PASSANT", "position": [-1, 0] }
]
```

### Available conditions

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
The piece at a relative position has never moved (`total_moves == 0`).

```json
{ "condition": "PIECE_FIRST_MOVE", "position": [3, 0] }
{ "condition": "PIECE_FIRST_MOVE", "position": [0, 0] }
{ "condition": "PIECE_FIRST_MOVE", "position": [3, 0], "absent_ok": true }
```

| Extra field | Default | Description |
|-------------|---------|-------------|
| `position` | — | `[dx, dy]` offset from the moving piece's source, written in neutral coordinates and rotated per-player at build time. `[0, 0]` checks the **moving piece itself** — i.e. this is the piece's first move. |
| `absent_ok` | `false` | When `true`, the condition passes vacuously if the target square is off-board or empty. Use this for castling where two conditions cover both possible rook positions; one will always be vacuous. |

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

#### `NOT_ATTACKED`
The target square is not in any opponent's attack set.

```json
{ "condition": "NOT_ATTACKED" }
{ "condition": "NOT_ATTACKED", "position": [0, 0] }
```

| Extra field | Default | Description |
|-------------|---------|-------------|
| `position` | move target | `[dx, dy]` offset from the **original source** to the square to check. When omitted the move target (`source + step`) is checked. Use `[0, 0]` to check whether the piece's current (source) square is attacked — useful for preventing castling while in check. |

---

#### `PATH_NOT_ATTACKED`
Every square from source up to and including the target is not in any opponent's attack set.
Used for castling's king path safety check.

No extra fields.

```json
{ "condition": "PATH_NOT_ATTACKED" }
```

---

#### `PATH_PIECE_COUNT`
Counts the pieces strictly between `original_source` and the current target and passes when that
count is within `[min, max]`. Optionally filtered to a single piece code.

```json
{ "condition": "PATH_PIECE_COUNT", "min": 1, "max": 1 }
{ "condition": "PATH_PIECE_COUNT", "piece": "CANNON", "max": 0 }
```

| Extra field | Default | Description |
|-------------|---------|-------------|
| `piece` | any | If set, only pieces with this code are counted. |
| `min` | `0` | Inclusive lower bound. |
| `max` | `255` | Inclusive upper bound. |

This condition is most useful on `repeat`-loop blueprints, where "path" grows with each
iteration. Classic use cases:
- Xiangqi/Janggi **cannon** — must jump over exactly one piece to capture
  (`"min": 1, "max": 1`).
- Janggi **cannon** — the screen piece must not itself be a cannon
  (`"piece": "CANNON", "max": 0`).

---

#### `PIECE_AT`
The piece at a relative position must have the specified code. Returns false if the position is
off-board or empty.

```json
{ "condition": "PIECE_AT", "position": [0, 1], "piece": "CANNON" }
```

| Extra field | Description |
|-------------|-------------|
| `position` | `[dx, dy]` relative to the moving piece's source, in neutral coordinates (rotated per-player at build time). |
| `piece` | The required piece code. |

---

#### `PIECE_NOT_AT`
The piece at a relative position must **not** have the specified code. Returns true (vacuously
passes) when the position is off-board or the square is empty.

```json
{ "condition": "PIECE_NOT_AT", "position": [0, 1], "piece": "CANNON" }
```

| Extra field | Description |
|-------------|-------------|
| `position` | `[dx, dy]` relative to the moving piece's source, in neutral coordinates. |
| `piece` | The forbidden piece code. |

Contrast with `PIECE_AT`: this condition passes when the occupant is absent, wrong code, or the
square is outside the board — making it suitable for "not a cannon in this slot" guards.

---

#### `LEG_EMPTY`
The intermediate "leg" square must be unoccupied. Used for Xiangqi-style horse movement where
the first orthogonal step must be clear before the piece can complete the L-shape.

```json
{ "condition": "LEG_EMPTY", "position": [0, 1] }
```

| Extra field | Description |
|-------------|-------------|
| `position` | `[dx, dy]` offset from the **source** to the leg square, in neutral coordinates (rotated per-player at build time). |

The condition passes vacuously when the leg position is off-board.

Unlike `PATH_PIECE_COUNT` (which counts pieces along the axis of the move step), `LEG_EMPTY`
checks an **explicit offset** regardless of the move's axis. This is necessary for L-shaped
jumps where the blocking square is not on the line between source and destination.

**Xiangqi horse example** — step `[1, 2]`, leg at `[0, 1]`:

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

---

#### `ALLY_ADJACENT_COUNT`
Counts the moving piece's ally pieces in the 8 orthogonal/diagonal squares surrounding the
source square. Passes when that count is within `[min, max]`. Optionally filtered to a piece
code.

```json
{ "condition": "ALLY_ADJACENT_COUNT", "min": 1 }
{ "condition": "ALLY_ADJACENT_COUNT", "piece": "STONE", "max": 2 }
```

| Extra field | Default | Description |
|-------------|---------|-------------|
| `piece` | any | If set, only allies with this code count. |
| `min` | `0` | Inclusive lower bound. |
| `max` | `255` | Inclusive upper bound. |

Example use: a "solidarity" rule where isolated pieces (no adjacent allies) cannot move — add
`{ "condition": "ALLY_ADJACENT_COUNT", "min": 1 }` to every move.

---

#### `ALLY_ON_FILE`
The current player already has an ally piece of the given code on the same file (column) as the
candidate square. Primarily used as a `drop_restriction` to implement Shogi's *nifu* rule (no
two unpromoted pawns on the same file).

```json
{ "condition": "ALLY_ON_FILE", "piece": "PAWN" }
```

| Extra field | Description |
|-------------|-------------|
| `piece` | The piece code to look for. The condition is true when an ally piece with this code exists on the same column. |

This condition is only meaningful inside `drop_restrictions`. It cannot fire during normal move
calculation (use `CHECK_STATE` for that).

---

#### Named global conditions (e.g. `REACH_END`, `RIVER_OR_BEYOND`)
Any code defined in the top-level `conditions` array can be referenced here by name. Currently
only `POSITION`-type conditions are supported. They can be used in:

- **Move-level `conditions`** — checked against the move's **target square** (`source + step`).
  Because lateral moves don't change the row, a condition like `RIVER_OR_BEYOND` on a lateral
  step effectively asks "is the piece already across the river?".
- **Modifier `conditions`** — checked against the move's landing square (same semantics).
- **`drop_restrictions`** — checked against the candidate drop square.

```json
{ "condition": "REACH_END" }
{ "condition": "RIVER_OR_BEYOND" }
```

Unknown codes that are **not** declared in the game's `conditions` array evaluate to `false`
(block the move). This is the safe default: an unrecognised condition restricts rather than
over-permits.

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

### `CONVERT`
Changes the owner of an enemy piece at a relative position to the current player. No-ops
when the square is empty, off-board, or already occupied by an ally.

```json
{ "action": "CONVERT", "target": [1, 0], "piece": "STONE" }
```

| Field | Description |
|-------|-------------|
| `target` | `[dx, dy]` relative to the **moving piece's source**, in neutral coordinates (rotated per-player at build time). |
| `piece` | Optional piece code the converted piece takes on. If omitted, the piece retains its original code. |

`CONVERT` fires per target position: add one entry per square you want to flip. This is how
Ataxx clone moves work — after moving/copying the stone to the target, seven `CONVERT` entries
cover every neighbouring square, turning any adjacent enemy stones to the current player.

The engine applies all `CONVERT` board changes atomically alongside the primary action, so the
converted pieces register as current-player pieces for subsequent rule checks in the same move.

---

### `COPY_SOURCE`
Places a fresh copy of the moving piece back at its original source position after the move.
Produces a "clone" effect: the source square ends up still occupied and the target square gains
a new piece.

No extra fields.

```json
{ "action": "COPY_SOURCE" }
```

Combine with a `MOVE` action (which vacates the source) and `COPY_SOURCE` (which replaces it):
net result is the source retains its piece and a new copy appears at the target. This is the
mechanism used for Ataxx clone moves.

```json
// Ataxx clone step [1, 0]
{
  "id": 0,
  "step": [1, 0],
  "actions": [{ "state": "EMPTY", "action": "MOVE" }],
  "side_effects": [
    { "action": "COPY_SOURCE" },
    { "action": "CONVERT", "target": [0, -1], "piece": "STONE" },
    { "action": "CONVERT", "target": [0,  1], "piece": "STONE" },
    ...
  ]
}
```

---

### `EXPLODE`
Removes the landing square and all adjacent squares within `radius` squares, optionally
exempting certain piece codes from destruction. Used for Atomic Chess.

```json
{ "action": "EXPLODE", "radius": 1, "exempt_codes": ["PAWN"] }
```

| Field | Default | Description |
|-------|---------|-------------|
| `radius` | `1` | Blast radius in squares (Chebyshev distance). All pieces within this distance of the landing square are removed, including allies, except those whose code is in `exempt_codes`. |
| `exempt_codes` | `[]` | Piece codes never destroyed by this explosion (e.g. pawns in Atomic Chess). |

The landing square is always cleared regardless of `exempt_codes`. Adjacent squares are cleared
unless the piece there has a code in `exempt_codes`.

**Atomic Chess example** — every capture action triggers an explosion that spares pawns:

```json
{
  "state": "ENEMY",
  "action": "CAPTURE",
  "side_effects": [{ "action": "EXPLODE", "exempt_codes": ["PAWN"] }]
}
```

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

Walk-through:

1. The king steps two squares right `[2, 0]` to an empty square.
2. Guards:
   - `PIECE_FIRST_MOVE{position:[0,0]}` — king must not have moved.
   - `NOT_ATTACKED{position:[0,0]}` — source square must not be attacked (can't castle while in check).
   - `PATH_PIECE_COUNT{max:0}` — squares between must be empty.
   - `NOT_ATTACKED` — destination must not be attacked.
   - `PATH_NOT_ATTACKED` — path must not be attacked.
3. Two `PIECE_FIRST_MOVE{absent_ok:true}` checks at `[3, 0]` and `[4, 0]` — one covers each
   player's rook position; `absent_ok` means the other passes vacuously (nothing is there).
4. The side effect teleports the matching rook next to the king.

---

## The hand

The **hand** is a per-player inventory of captured pieces available for reuse. It is the
mechanism behind Shogi drops and Crazyhouse drops.

It is enabled with `"hand_enabled": true` at the top level. When disabled (the default),
captures permanently remove pieces and the hand is never consulted.

### What the hand contains

When a player captures an opponent's piece, the captured piece's **demoted form** enters the
capturing player's hand:

- If the captured piece has `"demotes_to": null` (or the field is absent), the piece code itself
  enters the hand. A captured ROOK becomes a ROOK in hand.
- If the captured piece has `"demotes_to": "PAWN"`, a PAWN enters the hand instead. This is how
  promoted pieces revert to their base form — in Shogi a captured DRAGON becomes a ROOK in hand;
  in Crazyhouse a captured `P_QUEEN` becomes a PAWN in hand.

The hand is serialised with the rest of game state and survives save/restore.

Pieces can also be placed without a hand, from an unlimited supply: see
[Placing from a supply](#placing-from-a-supply).

### The demotion pattern

A key design decision is how to distinguish pieces that should demote on capture from pieces of
the same movement type that should not.

**Shogi**: every promoted piece is a distinct code (DRAGON, HORSE, TOKIN, …) with
`"demotes_to"` pointing to its base. The base pieces (ROOK, BISHOP, PAWN, …) have no
`"demotes_to"` and enter the hand as themselves.

**Crazyhouse**: this game requires that promoted pawns revert to pawns, but native queens do not
revert to anything. The solution is the same pattern: give promoted pawns their own codes
(`P_QUEEN`, `P_ROOK`, `P_BISHOP`, `P_KNIGHT`) with `"demotes_to": "PAWN"`, and let native
pieces (QUEEN, ROOK, etc.) carry no `"demotes_to"`. The pawn's `TRANSFORM` modifier then lists
the P_ codes rather than the native ones:

```json
{
  "code": "PAWN",
  "drop_restrictions": [{ "condition": "TERMINAL_RANK" }],
  "moves": [
    {
      "id": 0, "step": [0, 1],
      "actions": [{ "state": "EMPTY", "action": "MOVE" }],
      "modifiers": [{
        "action": "TRANSFORM",
        "conditions": [{ "condition": "REACH_END" }],
        "options": ["P_QUEEN", "P_ROOK", "P_BISHOP", "P_KNIGHT"]
      }]
    }
  ]
}
```

```json
{
  "code": "P_QUEEN",
  "name": "promoted pawn — queen",
  "demotes_to": "PAWN",
  "moves": [ /* identical to QUEEN */ ]
}
```

A native QUEEN captured from its starting square enters the hand as QUEEN. A `P_QUEEN` captured
after promotion enters the hand as PAWN. The engine has no special knowledge of either game's
rules — it just follows `demotes_to`.

### Drop move flow

1. The current player selects a piece code from their hand (`CalculateDrops`).
2. The engine computes all empty squares where the piece can legally be dropped, filtered by:
   - The piece's `drop_restrictions` — any restriction that fires excludes the square.
   - Self-check — squares are excluded if dropping there would leave the player's leader in check.
3. The engine enters the `Dropping` phase. Available drop squares are exposed the same way move
   squares are in the `Moving` phase.
4. The player selects a square (`ExecuteDrop`). A **fresh** piece is placed (zero `total_moves`,
   no state flags); the hand count decrements; the turn advances.

The freshness of dropped pieces has implications: a dropped pawn cannot immediately double-push
(it has no `FIRST_MOVE`), a dropped rook can participate in castling (its `total_moves == 0`
satisfies `ROOK_FIRST_MOVE` — this is the correct Crazyhouse behaviour).

### Drop restrictions

`drop_restrictions` is a list of conditions on the piece definition. Any condition that fires
for a candidate square **blocks** the drop there. Two types are commonly used:

**`ALLY_ON_FILE`** — blocks drops on files that already contain an ally piece of a given code.
Used for Shogi's *nifu* rule (no two unpromoted pawns on the same file).

**Named POSITION condition** — blocks drops on a specific set of squares. Use a symmetric
condition (identical lists for all players) when the restriction is absolute rather than
player-relative. Crazyhouse uses `TERMINAL_RANK` to block pawn drops on rows 0 and 7 for
both players; Shogi uses `PROMO_FORCED` (player-relative) to block drops where the piece
would immediately have no legal moves.

```json
// Shogi PAWN — player-relative "last rank" restriction + nifu
"drop_restrictions": [
  { "condition": "PROMO_FORCED" },
  { "condition": "ALLY_ON_FILE", "piece": "PAWN" }
]

// Crazyhouse PAWN — absolute terminal ranks restriction
"drop_restrictions": [
  { "condition": "TERMINAL_RANK" }
]
```

---

## Complete annotated example — Crazyhouse hand

Crazyhouse is standard chess with one rule addition: captured pieces enter the capturer's hand
and can be dropped back as that player's own piece. Promoted pawns revert to pawns on capture.

**Top-level additions over chess:**

```json
"hand_enabled": true,
"draw_conditions": { "repetition_count": 3 }
```

The fifty-move and insufficient-material draw conditions are omitted — in Crazyhouse the hand
makes material counts dynamic and the fifty-move rule rarely applies.

**Global condition — absolute terminal ranks:**

```json
{
  "code": "TERMINAL_RANK",
  "type": "POSITION",
  "check": {
    "WHITE": [[0,0],...,[7,0],[0,7],...,[7,7]],
    "BLACK": [[0,0],...,[7,0],[0,7],...,[7,7]]
  }
}
```

Both players share the identical list: rows 0 and 7 are blocked for pawn drops regardless of
orientation.

**Pawn — promotion options and drop restriction:**

```json
{
  "code": "PAWN",
  "drop_restrictions": [{ "condition": "TERMINAL_RANK" }],
  "moves": [
    {
      "id": 0, "step": [0, 1],
      "actions": [{ "state": "EMPTY", "action": "MOVE" }],
      "modifiers": [{
        "action": "TRANSFORM",
        "conditions": [{ "condition": "REACH_END" }],
        "options": ["P_QUEEN", "P_ROOK", "P_BISHOP", "P_KNIGHT"]
      }]
    },
    ...
  ]
}
```

The promotion options point to P_ codes rather than native codes.

**Promoted-pawn pieces:**

```json
{ "code": "P_QUEEN",  "name": "promoted pawn — queen",  "demotes_to": "PAWN", "moves": [ /* queen moves */ ] },
{ "code": "P_ROOK",   "name": "promoted pawn — rook",   "demotes_to": "PAWN", "moves": [ /* rook moves  */ ] },
{ "code": "P_BISHOP", "name": "promoted pawn — bishop", "demotes_to": "PAWN", "moves": [ /* bishop moves*/ ] },
{ "code": "P_KNIGHT", "name": "promoted pawn — knight", "demotes_to": "PAWN", "moves": [ /* knight moves*/ ] }
```

**Native pieces** (QUEEN, ROOK, BISHOP, KNIGHT) carry no `demotes_to` and enter the hand as
themselves when captured.

**Walk-through of a capture-demotion cycle:**

1. WHITE pawn reaches row 7, promotes to `P_QUEEN`. Board now has a `P_QUEEN` belonging to WHITE.
2. BLACK captures the `P_QUEEN`. Because `demotes_to: "PAWN"`, a PAWN enters BLACK's hand.
3. BLACK drops the PAWN on any empty square not on rows 0 or 7 (`TERMINAL_RANK` restriction).
4. The dropped pawn is a fresh piece: no state flags, `total_moves = 0`, owned by BLACK.

If instead BLACK had captured WHITE's native QUEEN (from starting position, no `demotes_to`),
a QUEEN would have entered BLACK's hand — and BLACK could drop a QUEEN anywhere empty.

---

## Placing from a supply

Some games hand each player as many pieces as they want: stones in Go, discs in Reversi. Mark
that piece with `"unlimited_supply": true`:

```json
{ "code": "STONE", "name": "stone", "unlimited_supply": true, "moves": [] }
```

Placing it works like a drop from the hand, with two differences: nothing needs to be in the
hand, and nothing is taken out of it. Any player may place it on any empty square, and
`drop_restrictions` still apply. `hand_enabled` is a separate switch: a game can have a supply
without a hand (Go), a hand without a supply (Shogi), or both.

A piece with a supply does not need `moves`; give it an empty list if it never moves. Placing a
piece is a turn, so it advances the turn, is recorded in the history as a `DROP`, and is
checked for self-check like any drop.

---

## Passing

`"allow_pass": true` lets the player to move give up their turn. The pass is recorded in the
history with the action `PASS` (and empty `from` and `to`). Because a player who may pass
always has a move, such a game is never a stalemate.

A pass is accepted when nothing is pending. A piece that was picked up is simply put back; a
promotion choice or a jump in progress has to be finished first.

Passing does not end a game by itself. Add an [`AREA_SCORE`](#area_score) win condition for
that, or the players can only stop by other means.

---

## Liberty capture

`liberty_capture` captures pieces that have been shut in. It is Go's capture rule, written so
that any game can use it.

```json
"liberty_capture": { "pieces": ["STONE"], "allow_suicide": false }
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `pieces` | yes | — | Piece codes that form groups. Pieces with any other code are never captured and are not liberties: they block the way, like the edge of the board. |
| `allow_suicide` | no | `false` | Whether a move may leave the mover's own group without liberties. |

Pieces of these codes that belong to one player and touch along a row or column form a **group**
(diagonals do not connect). The empty squares beside a group are its **liberties**. After every
move or placement:

1. Every group of the *other* players that has no liberty left is removed, all at once.
2. Then the mover's own groups are checked, counting the squares just freed as empty. A group
   with no liberty left is a **suicide**. With `allow_suicide` false (the default) such a move
   is not offered. With it true, the group is removed.

So a move that fills the last liberty of the mover's own group is still allowed if it first
captures something that gives the group room, which is how a Go capture works.

The rule runs for pieces that are placed and for pieces that move. Removed pieces count as
captured in the match view like any other captured piece (they do not enter a hand). A group that already has no liberties before the move, for instance in a
hand-built starting position, is only judged if the move touches it.

---

## Repeated positions

`"forbid_repeated_positions": true` makes a move illegal when it would leave the position as it
was after any earlier turn. The position is the board, the pieces' state flags and whose turn
it is. This is the *ko* rule of Go (as *situational superko*), which stops a capture and its
recapture from cycling forever.

It differs from `draw_conditions.repetition_count`, which lets the repetition happen and then
declares a draw. A move that is refused is simply left out of the legal moves, so a player can
be left with only `PASS`.

The starting position is not part of the history, so a move that would recreate it is not
refused. In a game where pieces are only added or captured (Go) that cannot happen.

---

## Complete annotated example — Go

`crates/engine/specs/go.json` is Go on a 9×9 board. Nothing in the engine knows about Go: each
rule is one of the switches above.

```json
{
  "name": "GO",
  "board": { "dimensions": [9, 9] },
  "players": [
    { "name": "BLACK", "direction": [[1, 0], [0, 1]], "starting_positions": [] },
    { "name": "WHITE", "direction": [[1, 0], [0, 1]], "starting_positions": [] }
  ],
  "turns": { "order": ["BLACK", "WHITE"] },
  "pieces": [
    { "code": "STONE", "name": "stone", "unlimited_supply": true, "moves": [] }
  ],
  "allow_pass": true,
  "forbid_repeated_positions": true,
  "liberty_capture": { "pieces": ["STONE"] },
  "win_conditions": [
    { "type": "AREA_SCORE", "pieces": ["STONE"], "territory": true, "bonus": { "WHITE": 7.5 } }
  ]
}
```

- The board starts empty (`starting_positions` is empty for both players) and the stone has no
  moves: a turn is a placement or a pass.
- `unlimited_supply` lets either player place a stone on any empty point.
- `liberty_capture` takes shut-in groups and refuses suicide. `forbid_repeated_positions` is ko.
- Two passes in a row end the game. Each player scores their stones plus the empty regions that
  only their stones touch, and WHITE gets 7.5 points for moving second, so the game cannot be
  drawn. For a 13×13 or a 19×19 board, change `dimensions`: `go_13.json` and `go_19.json` do.

---

## Known limitations

- **Promotion is always mandatory.** When a `TRANSFORM` modifier fires the player must choose an
  option; there is no "skip" path. To make promotion *optional*, include the piece's own code in
  the `options` list — choosing it replaces the piece with a fresh copy of itself.
- **`POSITION` conditions check the destination only.** A piece moving out of a special zone does
  not trigger a condition targeting that zone. Zone `restrictions` similarly only filter destination
  squares, not the source.
- **Zone restrictions don't apply to threats.** A piece with `restrictions` can still project
  threats outside its zone. This is intentional for the flying-general rule in Xiangqi but should
  be kept in mind for other designs.
- **`ALLY_ON_FILE` is drop-only.** This condition is only evaluated during drop legality checks,
  not during standard move generation.
- **`no_opponent_check` is a game-wide flag.** It cannot be set per-piece or per-move; all moves
  for all pieces are post-filtered when the flag is set.
- **`AREA_SCORE` has no dead-piece agreement.** It counts the board as it stands when the players
  pass, so a piece that is dead must be captured first. Scoring that removes dead pieces by
  agreement, or counts prisoners (Japanese rules), is not expressible.
- **Liberty groups connect along rows and columns only.** Boards whose points connect another way
  (Hex) cannot use `liberty_capture`.

---

## Removed primitives

The following condition codes have been removed from the engine. Use the canonical replacements
shown — the engine will block any move that references an unrecognised condition code.

| Removed | Replacement |
|---------|-------------|
| `FIRST_MOVE` | `{ "condition": "PIECE_FIRST_MOVE", "position": [0, 0] }` |
| `PATH_EMPTY` | `{ "condition": "PATH_PIECE_COUNT", "max": 0 }` |
| `ROOK_FIRST_MOVE` | `{ "condition": "PIECE_FIRST_MOVE", "position": [N, 0], "absent_ok": true }` |
| `SOURCE_NOT_ATTACKED` | `{ "condition": "NOT_ATTACKED", "position": [0, 0] }` |
| `OPPONENT_NOT_IN_CHECK` | Top-level `"no_opponent_check": true` game flag |

**Design principle**: never add a condition primitive that can be expressed by composing existing
ones.
