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
  "zones": [ ... ],
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
| `players` | yes | — | One entry per player with starting layout, and with `direction` on a grid (a map has none; see [Map boards](#map-boards)). |
| `turns` | yes | — | Turn order. |
| `zones` | no | `[]` | Named sets of places, one list per player, used by `IN_ZONE` tests and win conditions. See [`zones`](#zones). |
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
| `dimensions` | for square | `[cols, rows]` — both must be ≥ 1. Positions are 0-indexed: `[0,0]` is the bottom-left. |
| `disabled_positions` | no | Squares that don't exist (think of hollow boards or irregular shapes). Pieces cannot be placed on or moved to disabled positions. |

### Hex boards

```json
"board": { "shape": "hex", "layout": "hexagon", "size": 6 }
```

| Field | Required | Description |
|-------|----------|-------------|
| `shape` | no | `"square"` (the default, with `dimensions` as above) or `"hex"`. A map takes `"map"` or `"hex-map"` (see [Map boards](#map-boards)). |
| `layout` | for hex | `"rhombus"`: the `size × size` grid. `"hexagon"`: a hexagon with `size` cells per side, which is the `(2·size − 1) × (2·size − 1)` grid with the cells where `|x − y| ≥ size` cut away (side 6 gives 91 cells). |
| `size` | for hex | At least 1. A hex board does not take `dimensions`. |
| `disabled_positions` | no | Further holes, as on a square board. |

A hex cell `[x, y]` has six neighbours: `[x, y ± 1]`, `[x ± 1, y]`, `[x + 1, y + 1]` and
`[x − 1, y − 1]`. They are both the `orthogonal` and the `adjacent` neighbourhood, so liberties,
explosions, ally counts and `CONNECTION` use them. Steps, relative positions and `direction`
matrices work as on squares: `[0, 1]` is one cell straight up a file, and `[[-1, 0], [0, -1]]`
turns a player half way round.

### Map boards

A map is a list of named places joined by links, for games that are not grids: dots and lines
(Pong hau k'i), or the corners, sides and tiles of a Catan board.

```json
"board": {
  "shape": "map",
  "places": [
    { "name": "a", "at": [0, 0] },
    { "name": "b", "at": [2, 0] },
    { "name": "ab", "kind": "path", "ends": ["a", "b"] },
    { "name": "c", "at": [1, 2] },
    { "name": "f", "kind": "area", "around": ["a", "b", "c"] }
  ],
  "links": [["a", "c"]]
}
```

| Field of a place | Required | Description |
|------------------|----------|-------------|
| `name` | yes | 1 to 64 bytes, unique. A map has 1 to 10,000 places. |
| `kind` | no | `"point"` (the default), `"path"` or `"area"`. Every kind can hold a piece. |
| `at` | for a point | Where it is drawn, in any units. A path sits at the middle of its ends and an area at the average of its corners. |
| `ends` | for a path | Two different points. |
| `around` | for an area | Three or more different points, in drawing order. |

`links` is optional: pairs of points joined by a line, each both ways. A path joins its two
ends without one.

`"shape": "hex-map"` makes a map from a hex layout, with the same `layout`, `size` and
`disabled_positions` as a hex board. Each tile is an area named `"q,r"`, each side is a path
named by its two tiles (`"2,2|2,3"`), and each corner is a point named by its three tiles
(`"1,1|1,2|2,2"`). Tiles are sorted by `q`, then `r`, and joined by `|`. Off-board tiles count,
so a name can hold a negative number (`"-1,0"`). A size-3 hexagon has 19 tiles, 54 corners and
72 sides.

A move on a map names a link label as its `step`: `"step": "next"`. It goes to every place the
label reaches, one move for each.

| Label | From | To |
|-------|------|----|
| `next` | a point | the points joined to it by a path or a written link |
| `next` | a path | the paths that share one of its ends |
| `next` | an area | the areas that share a side with it |
| `ends` | a path | its two points |
| `paths` | a point | the paths that end at it |
| `corners` | an area | the points around it |
| `areas` | a point or path | the areas it lies on |
| `sides` | an area | the paths along its edge |

On a map, `orthogonal` and `adjacent` both mean `next`, and a test's `AROUND` may go `through`
any of the labels above. Positions are place names, in `starting_positions` and in `zones` (a
name works on a grid too, as `"4,1"`). A player has no `direction`: leave it out.

A map refuses, with an error that names the problem: a vector `step`, or side effect `from`,
`to` or `target`; a test's `offset`, `PATH` or `LINE`; `repeat`; a `direction`; coordinates
where a place is due; a label no map has; and, on a grid, a label `step`.

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

Must be a valid rotation (or rotation+reflection): `det(matrix) = ±1`. A map has no orientation: leave `direction` out (see [Map boards](#map-boards)).

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
  { "type": "AREA_SCORE", "pieces": ["STONE"], "territory": true, "bonus": { "WHITE": 7.5 } },
  { "type": "CONNECTION", "through": "adjacent", "zones": ["SIDE_A", "SIDE_B"], "pieces": ["STONE"] }
]
```

### `PIECE_IN_ZONE`

The moving player wins when any of their pieces with the given code land in the named zone
(see [`zones`](#zones)).

```json
{ "type": "PIECE_IN_ZONE", "piece": "KING", "zone": "CENTER" }
```

| Field | Required | Description |
|-------|----------|-------------|
| `piece` | yes | Code of the piece that must reach the zone (e.g. `"KING"`). |
| `zone` | yes | Name of a zone that defines the winning squares for each player. |

The zone must be declared under `zones`; a name that is not is refused when the spec is read.
Because a zone lists its squares per player, "reach your opponent's back rank" is
straightforward.

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

### `CONNECTION`

```json
{ "type": "CONNECTION", "through": "adjacent", "zones": ["SIDE_A", "SIDE_B"], "at_least": 2, "pieces": ["STONE"] }
```

The moving player wins when **one chain** of their pieces touches at least `at_least` of the
`zones`. Two pieces belong to the same chain when they are neighbours through `through`.

| Field | Required | Description |
|-------|----------|-------------|
| `through` | yes | `"orthogonal"` or `"adjacent"`: the neighbourhood that joins a chain. On a hex board both are the six cells around a cell. |
| `zones` | yes | Names of zones declared under `zones`. Each is read with the moving player's own cells, so every player can have their own sides. |
| `at_least` | no | How many of the zones one chain must touch. Defaults to all of them. |
| `pieces` | no | The piece codes a chain is made of. Empty means every piece. |

Hex is one `CONNECTION` with two zones, `SIDE_A` and `SIDE_B`, where each player's cells are
their own two sides of the board (`hex.json`).

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

## `zones`

A zone is a named set of places, with one list for each player. Tests ask about zones with
`IN_ZONE` (see [Tests](#tests)), and win conditions name them (`PIECE_IN_ZONE`, `CONNECTION`).

```json
"zones": [
  {
    "name": "REACH_END",
    "places": {
      "WHITE": [[0,7],[1,7],[2,7],[3,7],[4,7],[5,7],[6,7],[7,7]],
      "BLACK": [[0,0],[1,0],[2,0],[3,0],[4,0],[5,0],[6,0],[7,0]]
    }
  }
]
```

| Field | Required | Description |
|-------|----------|-------------|
| `name` | yes | The name tests and win conditions use. Two zones may not share a name. |
| `places` | yes | For each player name, the places of that player's part of the zone. On a grid a place is `[x, y]` in **board coordinates** (absolute, not turned by `direction`) or a square's name such as `"4,1"`; on a map it is a place name. |

A place is in the zone for a player when it is in that player's list. A player the zone does not
list has no part of it.

Use zones for promotion rows, palaces, halves of the board, scoring squares, or any set of places
a rule needs to name.

**Same places for everyone**: when every player gets an identical list, the zone is the same set
of squares whatever the direction of play. For example, both end rows of the board, where no
pawn may be dropped:

```json
{
  "name": "TERMINAL_RANK",
  "places": {
    "WHITE": [[0,0],[1,0],[2,0],[3,0],[4,0],[5,0],[6,0],[7,0],[0,7],[1,7],[2,7],[3,7],[4,7],[5,7],[6,7],[7,7]],
    "BLACK": [[0,0],[1,0],[2,0],[3,0],[4,0],[5,0],[6,0],[7,0],[0,7],[1,7],[2,7],[3,7],[4,7],[5,7],[6,7],[7,7]]
  }
}
```

Contrast this with `REACH_END` above, where the lists differ because "the last rank" depends on
the player.

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
    "drop_conditions": [ ... ]
  }
]
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `code` | yes | — | Unique identifier used everywhere else (starting positions, promotions, etc.). |
| `name` | no | — | Human-readable label. Not used by the engine. |
| `moves` | yes | — | List of move definitions (see below). |
| `demotes_to` | no | `null` | When `hand_enabled` is true and this piece is captured, the piece code that enters the capturer's hand. `null` means the piece enters the hand as itself. Set this on promoted pieces to make them revert to their base form on capture — e.g. `"demotes_to": "PAWN"` on a Shogi TOKIN or a Crazyhouse `P_QUEEN`. See the hand section for the full demotion pattern. |
| `unlimited_supply` | no | `false` | When `true`, any player may place this piece on an empty square at any time, as often as they like, without it ever being in a hand. See [Placing from a supply](#placing-from-a-supply). |
| `drop_conditions` | no | `[]` | [Tests](#tests) that must all hold for the piece to be dropped on a square. `TARGET` is that square; `START` is refused, as a drop has no moving piece. See [Drop conditions](#drop-conditions). |
| `value` | no | — | What the piece is worth in the material count shown beside a match. Pieces with no value count as 0, and a game in which no piece has a value shows no material at all. |

The CLI renders the first three characters of `code` inside each board cell, so keep codes
descriptive enough that the three-character prefix is unambiguous.

### Keeping a piece in a zone

To keep a piece inside a zone, give each of its moves an `IN_ZONE` test on the landing square.
A Xiangqi GENERAL never leaves its palace:

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

A move's tests decide which moves are offered. They do not change what the piece attacks (see
[`threat_only`](#threat_only)), so a GENERAL kept in its palace still attacks squares outside it.

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
| `id` | yes | Integer unique within this piece. Errors name a move by it. |
| `step` | yes | `[dx, dy]` offset applied to the piece's position. Written from the neutral player's perspective; the engine rotates it per-player using the direction matrix. On a map it is a link label instead, such as `"next"` (see [Map boards](#map-boards)). |
| `players` | no | The players who may make this move. Empty (the default) means every player. |
| `threat_only` | no | When `true`, this blueprint contributes to the piece's **attack map** but never produces legal moves. Default `false`. See [threat_only](#threat_only) below. |
| `actions` | yes | What happens depending on the square's occupancy (see below). |
| `conditions` | no | [Tests](#tests) that must all hold, or the move is not offered. |
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

**Important**: the attack map does not judge a move's tests. A move's `conditions` and its
`repeat.until` only decide which moves are offered; for the attack map, a move follows its step
and ends at the first piece. So a GENERAL kept in its palace by `IN_ZONE` tests still projects
the `threat_only` sliding blueprint across the entire file.

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

An action can have its own list of [tests](#tests). These are checked *after* the move-level
conditions and only for that specific action. Useful for conditional captures (en passant) where the same step
can be a normal move or a special capture depending on board state.

```json
{
  "state": "EMPTY",
  "action": "MOVE",
  "conditions": [
    { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START", "offset": [-1, 0] } }, "where": { "flag": "EN_PASSANT" } }
  ],
  "side_effects": [ ... ]
}
```

### Action-level `side_effects`

Side effects attached to a specific action only fire when that action is taken. See the
[side effects](#side-effects) section.

---

## Tests

A rule's conditions are **tests**: small blocks that each hold or not, put together into a
tree. A test looks at a **region** of the board (one or more **places**) and counts the pieces
there that pass a **filter**, or asks whether those places are attacked, or whether a place is in
a zone.

```json
"conditions": [
  { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START" } }, "where": { "moved": false } },
  { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START", "offset": [0, 1] } } } }
]
```

This is the chess pawn's double step: the pawn on `START` has never moved, and no piece stands on
the square in front of it.

### Where tests appear

| Where | Holds when | `START` | `TARGET` |
|-------|------------|---------|----------|
| A move's `conditions` | every test holds; otherwise the move is not offered | the moving piece's square | the square the move lands on (for a slide, the square being judged) |
| An action's `conditions` | every test holds; checked after the move's, for that action only | as above | as above |
| A modifier's `conditions` | every test holds; otherwise the modifier does not fire | as above | the landing square |
| A piece's `drop_conditions` | every test holds; otherwise the piece may not be dropped there | refused | the square being dropped on |
| A repeat's `until` | one test; when it holds the slide goes no further | the moving piece's square | the square the slide has just reached |

### Tests

| Test | Written | True when |
|------|---------|-----------|
| `ALL` | `{ "test": "ALL", "of": [T, ...] }` | every test in `of` holds (an empty list holds) |
| `ANY` | `{ "test": "ANY", "of": [T, ...] }` | at least one test in `of` holds (an empty list does not) |
| `NOT` | `{ "test": "NOT", "of": T }` | `of` does not hold |
| `COUNT` | `{ "test": "COUNT", "in": R, "where": F, "min": 1, "max": n }` | the number of pieces in region `R` that pass filter `F` is at least `min` and at most `max`. `where` defaults to every piece, `min` to `1`, `max` to no limit |
| `ATTACKED` | `{ "test": "ATTACKED", "in": R, "by": "ENEMY" }` | some place in `R` is attacked by an opponent of the mover, by the same attack map that check uses. `by` is `ENEMY`, the default and only value. A region with no places is not attacked |
| `IN_ZONE` | `{ "test": "IN_ZONE", "at": P, "zone": "PALACE" }` | place `P` is in the mover's part of the named zone (see [`zones`](#zones)) |

Because `min` defaults to `1`, a `COUNT` with nothing else written means "at least one piece is
there". For "no piece is there", write `"min": 0, "max": 0`, or wrap the `COUNT` in a `NOT`.
`"max": 0` alone is refused: its `min` would be `1`, above its `max`.

### Regions

| Region | Written | Places |
|--------|---------|--------|
| `AT` | `{ "region": "AT", "at": P }` | the one place `P` |
| `PATH` | `{ "region": "PATH", "from": P, "to": P, "ends": ["FROM", "TO"] }` | the places on the straight line between `from` and `to`, without its ends unless `ends` names them. `ends` defaults to none. Grids only |
| `AROUND` | `{ "region": "AROUND", "of": P, "through": "adjacent" }` | the neighbours of `P` through `adjacent` (the default) or `orthogonal`; on a map, `through` may also be one of the map's labels (see [Map boards](#map-boards)) |
| `LINE` | `{ "region": "LINE", "through": P, "along": [0, 1] }` | every square on the board's line through `P` in the direction `along`, both ways, `P` included. Disabled squares are skipped; the line stops only at the edge of the board. Grids only |

A straight line steps by -1, 0 or +1 on each axis. `PATH` walks one such step from `from` until it
reaches `to`. `LINE`'s `along` must be such a step (and not `[0, 0]`); like an offset, it is
written from the neutral player's view and turned for each player.

### Places

A place is `{ "place": "START" }` or `{ "place": "TARGET" }` (see the table above for what each
names), with an optional `"offset": [dx, dy]`. The offset is written from the neutral player's
view and turned for each player, like a `step`. Offsets are for grids only.

```json
{ "place": "START", "offset": [-1, 0] }
```

is the square beside the moving piece: to its left for a player who moves up the board, to its
right for one who moves down.

### Filters

A filter (`where`) is an object whose fields must all hold. Each field is optional.

| Field | Default | A piece passes when |
|-------|---------|---------------------|
| `owner` | `"ANY"` | it is the mover's (`"ALLY"`), an opponent's (`"ENEMY"`), or anyone's (`"ANY"`) |
| `codes` | every code | its code is one of these, as in `["PAWN", "LANCE"]` |
| `flag` | — | it carries this state flag (see [`SET_STATE`](#set_state)) |
| `moved` | — | it has moved at least once (`true`), or never (`false`) |

### A region that cannot be formed

A region cannot be formed when its place is off the board, or when a `PATH` is not a straight
line, or crosses a hole or the edge of the board. `COUNT` and `ATTACKED` over such a region are
**false**, and `NOT` flips that as usual. So `NOT` of a `COUNT` on a square off the board holds:
a Xiangqi horse whose leg square would be off the board is not blocked by it. `IN_ZONE` on a
place off the board is false.

### Checked when the spec is read

A test that names something the game does not have is refused when the spec is read, never read
as false. The error names the problem and where it is, as in
``InvalidTest("an IN_ZONE in a modifier of move 0 of piece PADATI names the zone `REACH_ED`, which `zones` does not declare")``.

| Refused | Error |
|---------|-------|
| An unknown test, region or field, such as `"test": "CONT"` or an `owner` written beside `in` | `JsonError`, with `unknown variant` or `unknown field` |
| A place other than `START` or `TARGET`; `START` in a drop condition | `InvalidTest` |
| `min` above `max` (including `"max": 0` with no `min`) | `InvalidTest` |
| An `owner` other than `ALLY`, `ENEMY`, `ANY`; a `by` other than `ENEMY`; an end other than `FROM`, `TO` | `InvalidTest` |
| A code in `codes` that no piece has | `InvalidTest` |
| A zone that `zones` does not declare | `InvalidTest` (in a test), `InvalidWinCondition` (in a win condition) |
| An `AROUND` label the board does not have | `InvalidTest` on a grid, `UnknownLabel` on a map |
| A `LINE` along `[0, 0]`, or along a step bigger than 1 on an axis | `InvalidTest` |
| An offset, `PATH` or `LINE` on a map | `NotOnAMap` |
| An offset or `along` with the wrong number of entries, or too large | `MismatchedDimensions`, `ValueOutOfRange` |

### Examples

The Xiangqi horse may not leap if a piece stands on its first step, its "leg":

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

The Xiangqi general keeps to its palace. Every one of its moves carries the test:

```json
{ "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "PALACE" }
```

The cannon captures only over exactly one piece, of either side:

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

In Solidarity chess a piece moves only with a friend beside it:

```json
{ "test": "COUNT", "in": { "region": "AROUND", "of": { "place": "START" } }, "where": { "owner": "ALLY" }, "min": 1 }
```

Shogi's *nifu*: no pawn may be dropped on a file that already holds one of your unpromoted pawns.

```json
"drop_conditions": [
  { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "LINE", "through": { "place": "TARGET" }, "along": [0, 1] }, "where": { "owner": "ALLY", "codes": ["PAWN"] } } }
]
```

---

## `repeat`

Makes the move step until it hits something or a limit is reached. This is how sliding pieces
(rooks, bishops, queens) are modelled.

```json
"repeat": {
  "until": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } } },
  "loop": true
}
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `until` | no | the next square is occupied | A [test](#tests), judged on each square the slide reaches, with `TARGET` that square and `START` the moving piece's square. The square is still judged as a move (its action decides, so an enemy there can be captured); when `until` holds, the slide goes no further. The default is the `COUNT` above: stop at the first piece. |
| `loop` | no | `false` | If true, repeat indefinitely until `until` triggers or the edge of the board. |
| `times` | no | `1` | Maximum repetitions when `loop` is false. |

**Typical patterns:**

```json
// Slides until blocked (rook, bishop, queen)
"repeat": { "loop": true }

// Exactly two steps (pawn double-push)
"repeat": { "times": 2 }

// Slides to the edge, past every piece (the Xiangqi cannon's capture)
"repeat": { "until": { "test": "ANY", "of": [] }, "loop": true }
```

An `ANY` of an empty list never holds, so that slide never stops early.

---

## `modifiers`

Post-move hooks that can change the move's outcome. The only supported modifier action is
`TRANSFORM` (promotion).

```json
"modifiers": [
  {
    "action": "TRANSFORM",
    "conditions": [{ "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "REACH_END" }],
    "options": ["QUEEN", "ROOK", "BISHOP", "KNIGHT"]
  }
]
```

| Field | Required | Description |
|-------|----------|-------------|
| `action` | yes | Must be `"TRANSFORM"`. |
| `conditions` | no | [Tests](#tests) that must all hold for the modifier to fire. `TARGET` is the landing square, `START` the square the piece left. |
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

State flags are read by a `COUNT` test whose filter names the `flag`. They're the mechanism for tracking
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

- All `[dx, dy]` values in `step`, a test's `offset` and `along`, and side effect
  `from`/`to`/`target` are written in **neutral coordinates** (as if the player moves in the +y
  direction).
- The engine rotates them per-player using the direction matrix at build time.
- Absolute positions in `starting_positions` and in the `places` of `zones` are written in
  **board coordinates** (origin at bottom-left).

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
        {
          "test": "COUNT",
          "in": { "region": "AT", "at": { "place": "START", "offset": [-1, 0] } },
          "where": { "flag": "EN_PASSANT" }
        }
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
3. If the target square is **empty**, count the pieces on the square at `[-1, 0]` from the pawn
   (beside it) that carry the `EN_PASSANT` flag. The `COUNT` holds when there is at least one.
4. If yes: move to the empty square *and* remove the pawn at `[-1, 0]` as a side effect.
5. If no: the action is blocked and the move is not offered.

The `EN_PASSANT` flag is set by a separate pawn move's `SET_STATE` side effect when a pawn
advances two squares.

---

## Complete annotated example — kingside castling

White's kingside castle, from `chess.json`:

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

Walk-through:

1. The king steps two squares right `[2, 0]` to an empty square. Only White has this move
   (`players`): Black's rook stands at a different distance, so Black has moves of its own.
2. Guards:
   - `COUNT` at `START` where `moved` is `false` — the king has never moved.
   - `NOT` `ATTACKED` at `START` — the king is not in check.
   - `COUNT` in the `PATH` to `[3, 0]`, `min` and `max` `0` — the squares between king and rook
     are empty.
   - `NOT` `ATTACKED` at `TARGET` — the landing square is safe.
   - `NOT` `ATTACKED` in the `PATH` from `START` to `TARGET`, with its `TO` end — every square
     the king crosses or lands on is safe.
   - Two `COUNT`s at `[3, 0]` — an allied rook stands there, and it has never moved.
3. The side effect moves the rook next to the king.

If the rook square were off the board, both `COUNT`s on it would be false and the move would not
be offered.

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
  "drop_conditions": [
    { "test": "NOT", "of": { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "TERMINAL_RANK" } }
  ],
  "moves": [
    {
      "id": 0, "step": [0, 1],
      "actions": [{ "state": "EMPTY", "action": "MOVE" }],
      "modifiers": [{
        "action": "TRANSFORM",
        "conditions": [{ "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "REACH_END" }],
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
   - The piece's `drop_conditions` — a square where any test fails is excluded.
   - Self-check — squares are excluded if dropping there would leave the player's leader in check.
3. The engine enters the `Dropping` phase. Available drop squares are exposed the same way move
   squares are in the `Moving` phase.
4. The player selects a square (`ExecuteDrop`). A **fresh** piece is placed (zero `total_moves`,
   no state flags); the hand count decrements; the turn advances.

The freshness of dropped pieces has implications: a dropped piece has never moved, so it passes
every `"moved": false` test. A pawn dropped on its second rank may double-push, and a dropped rook
can participate in castling — this is the correct Crazyhouse behaviour.

### Drop conditions

`drop_conditions` is a list of [tests](#tests) on the piece definition. A piece may be dropped
on an empty square only when **every** test holds there. `TARGET` is the square being dropped
on; `START` is refused, as a drop has no moving piece.

Two kinds are common:

**Not in a zone** — `NOT` of an `IN_ZONE` keeps drops off a set of squares. Use a zone with
identical lists for all players when the squares are the same for everyone. Crazyhouse uses
`TERMINAL_RANK` to keep pawn drops off rows 0 and 7 for both players; Shogi uses `PROMO_FORCED`
(different for each player) to keep drops off squares where the piece could never move again.

**Not on a file with a pawn** — `NOT` of a `COUNT` over the `LINE` through `TARGET` along
`[0, 1]`, the file. This is Shogi's *nifu* rule (no two unpromoted pawns on the same file).

```json
// Shogi PAWN — player-relative "last rank" zone + nifu
"drop_conditions": [
  { "test": "NOT", "of": { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "PROMO_FORCED" } },
  { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "LINE", "through": { "place": "TARGET" }, "along": [0, 1] }, "where": { "owner": "ALLY", "codes": ["PAWN"] } } }
]

// Crazyhouse PAWN — the same terminal ranks for both players
"drop_conditions": [
  { "test": "NOT", "of": { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "TERMINAL_RANK" } }
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

**Zone — the same terminal ranks for both players:**

```json
{
  "name": "TERMINAL_RANK",
  "places": {
    "WHITE": [[0,0],...,[7,0],[0,7],...,[7,7]],
    "BLACK": [[0,0],...,[7,0],[0,7],...,[7,7]]
  }
}
```

Both players share the identical list: rows 0 and 7 are blocked for pawn drops regardless of
orientation.

**Pawn — promotion options and drop condition:**

```json
{
  "code": "PAWN",
  "drop_conditions": [
    { "test": "NOT", "of": { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "TERMINAL_RANK" } }
  ],
  "moves": [
    {
      "id": 0, "step": [0, 1],
      "actions": [{ "state": "EMPTY", "action": "MOVE" }],
      "modifiers": [{
        "action": "TRANSFORM",
        "conditions": [{ "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "REACH_END" }],
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
3. BLACK drops the PAWN on any empty square not on rows 0 or 7 (its `TERMINAL_RANK` drop condition).
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
`drop_conditions` still apply. `hand_enabled` is a separate switch: a game can have a supply
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
- **Tests don't apply to threats.** A move's `conditions` and its `repeat.until` decide which
  moves are offered, not what the piece attacks. A piece kept in a zone by `IN_ZONE` tests can
  still project threats outside it. This is intentional for the flying-general rule in Xiangqi
  but should be kept in mind for other designs.
- **`ATTACKED` has one attacker, `ENEMY`.** A test cannot ask whether a square is defended by the
  mover's own pieces.
- **Offsets, `PATH` and `LINE` are for grids only.** On a map, tests reach other places through
  `AROUND` and the map's labels.
- **`no_opponent_check` is a game-wide flag.** It cannot be set per-piece or per-move; all moves
  for all pieces are post-filtered when the flag is set.
- **`AREA_SCORE` has no dead-piece agreement.** It counts the board as it stands when the players
  pass, so a piece that is dead must be captured first. Scoring that removes dead pieces by
  agreement, or counts prisoners (Japanese rules), is not expressible.
- **Liberty groups connect along rows and columns only.** Boards whose points connect another way
  (Hex) cannot use `liberty_capture`.

---

## Old forms

Earlier versions of the format wrote rules as named conditions. Each old form is refused when the
spec is read, with an `OldForm` error that names the form and what replaced it:

| Old form | Write instead |
|----------|---------------|
| The top-level `conditions` list of named zones | `zones`: `{ "name": "PALACE", "places": { "RED": [...], "BLACK": [...] } }` |
| A condition object, with a `condition` field naming it | a [test](#tests), `{ "test": "...", ... }` |
| A piece's `restrictions` | an `IN_ZONE` test at `TARGET` on each of its moves |
| A piece's drop restrictions, which blocked a drop when one held | `drop_conditions`, which allow a drop when they all hold |
| A repeat's `until` written as a name, such as `"NEVER"` | a test; leave it out to stop at the first piece, or write `{ "test": "ANY", "of": [] }` never to stop early |
| A side effect's `condition` | a test in the move's or the action's `conditions` |

The rules the old conditions wrote, as blocks. Here `COUNT in AT(START+[0,1])` is short for
`{ "test": "COUNT", "in": { "region": "AT", "at": { "place": "START", "offset": [0, 1] } } }`:

| Rule | As blocks |
|------|-----------|
| a leg square is empty (the Xiangqi horse) | `NOT(COUNT in AT(START+[0,1]))` |
| the square in front is empty (pawn double step) | `NOT(COUNT in AT(START+[0,1]))` |
| an enemy piece on a square (the draughts jump) | `COUNT in AT(START+[1,1]) where owner ENEMY` |
| no cannon on the square (Janggi cannon, on a slide) | `NOT(COUNT in AT(TARGET) where codes [CANNON])` |
| the moving piece has never moved | `COUNT in AT(START) where moved false` |
| a piece on a square has never moved, or the square is empty | `NOT(COUNT in AT(START+[3,0]) where moved true)` |
| the piece beside carries a flag (en passant) | `COUNT in AT(START+[-1,0]) where flag EN_PASSANT` |
| the landing square is not attacked | `NOT(ATTACKED in AT(TARGET))` |
| the piece's own square is not attacked (not in check) | `NOT(ATTACKED in AT(START))` |
| no square the piece crosses or lands on is attacked | `NOT(ATTACKED in PATH(START, TARGET, ends [TO]))` |
| nothing between the piece and the square | `COUNT in PATH(START, TARGET) min 0 max 0` |
| exactly one piece between (the cannon's screen) | `COUNT in PATH(START, TARGET) min 1 max 1` |
| at least one ally next to the piece (Solidarity) | `COUNT in AROUND(START) where owner ALLY` |
| no allied pawn on the file (*nifu*, for a drop) | `NOT(COUNT in LINE(TARGET, [0,1]) where owner ALLY, codes [PAWN])` |
| the move lands in a zone | `IN_ZONE(TARGET, REACH_END)` |
| no drop in a zone | `NOT(IN_ZONE(TARGET, TERMINAL_RANK))` in `drop_conditions` |
| a piece stays in a zone | `IN_ZONE(TARGET, PALACE)` on each of its moves |

**Design principle**: never add a test that can be expressed by composing existing ones.
