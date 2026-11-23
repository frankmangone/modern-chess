# Game Spec Format

A game is fully described by a single JSON file. The engine reads it, validates it, and runs the
game without any hardcoded rules. This document explains every field.

---

## Top-level structure

```json
{
  "name": "CHESS",
  "bags": [ ... ],
  "board": { ... },
  "players": [ ... ],
  "turns": { ... },
  "zones": [ ... ],
  "actions": [ ... ],
  "every_action": [ ... ],
  "must": [ ... ],
  "counters": [ ... ],
  "end_checks": [ ... ],
  "score": [ ... ],
  "pieces": [ ... ]
}
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `name` | yes | — | Human-readable game name. |
| `bags` | no | `[]` | The stores off the board: each player's hand, a shared supply. A game with no `bags` has no hand. What goes into a bag is written on the effects that remove pieces (`removed_to`), and a `HELD` choice takes a piece out. See [Bags and the hand](#bags-and-the-hand). Replaces `hand` and `hand_enabled`. |
| `counters` | no | `[]` | Names of the numbers each player keeps, changed by the `ADD` and `RESET` effects and read by `COUNTER`. See [`counters`](#counters). |
| `end_checks` | no | `[]` | Tests judged in order after every turn; the first that holds ends the game or removes a player. A game with none ends only when nobody can be left to play. See [`end_checks`](#end_checks). |
| `score` | no | `[]` | A list of terms summed for each player, read by the `HIGHEST` outcome and shown live. See [`score`](#score). |
| `board` | yes | — | Board geometry. |
| `players` | yes | — | One entry per player with starting layout, and with `direction` on a grid (a map has none; see [Map boards](#map-boards)). |
| `turns` | yes | — | Turn order, and the stages a turn may go on in. See [`turns`](#turns) and [Stages](#stages). |
| `zones` | no | `[]` | Named sets of places, one list per player, used by `IN_ZONE` tests. See [`zones`](#zones). |
| `actions` | yes | — | What a player may do on their turn: a piece move, a drop, a placing, a pass. At least one. See [`actions`](#actions). |
| `every_action` | no | `[]` | Steps run after the own steps of every action, or of the actions named: check, liberty capture, ko. See [`every_action`](#every_action). |
| `must` | no | `[]` | Tests that narrow the ways of playing: if some way passes one, only those stay (a forced capture). See [`must`](#must). |
| `pieces` | yes | — | All piece types and where each can go. |

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
`[x − 1, y − 1]`. They are both the `orthogonal` and the `adjacent` neighbourhood, so groups,
explosions, ally counts and `GROUP` use them. Steps, relative positions and `direction`
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
| `capacity` | no | How many pieces the place holds, at least 1; `1` when not written. Above 1 the place keeps counts of pieces: see [Places that hold many, and Kalah](#places-that-hold-many-and-kalah). |
| `start` | no | What a place that holds many has when the game starts, as `{ "STONE": 4 }`. Only with a `capacity` above 1. |

`links` is optional: pairs of points joined by a line, each both ways. A path joins its two
ends without one. A link may also be written `{ "from": "a", "to": "b" }`: it goes one way only,
from `a` to `b`, under the label `next` (see [One-way links](#one-way-links)). Both ends of a link
are different points; a link that names anything else is refused with `InvalidMap`.

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
| `next` | a point | the points joined to it by a path or a written link (a one-way link only from its `from`) |
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

A map refuses, with an error that names the problem: a vector `step`; an `offset` on a place, in
a test or an effect; a test's `PATH` or `LINE`; `repeat`; a `direction`; coordinates where a place
is due; a label no map has; and, on a grid, a label `step`. A `BOARD` region (see
[Regions](#regions)) works on maps too.

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

## `end_checks`

A top-level list of rules for how a game ends. They are judged in order after every turn, once the
turn has passed to the next player. The first check whose test holds ends the game or removes a
player, and no later check runs. Chess ends like this (`chess.json`; its insufficient-material
check is left out here):

```json
"end_checks": [
  { "when": { "test": "REPEATS", "times": 3 }, "then": { "outcome": "DRAW" } },
  { "when": { "test": "COUNTER", "name": "QUIET", "min": 100 }, "then": { "outcome": "DRAW" } },
  { "as": "NEXT",
    "when": { "test": "ALL", "of": [
      { "test": "STUCK" },
      { "test": "ATTACKED", "in": { "region": "BOARD", "where": { "owner": "ALLY", "codes": ["KING"] } }, "by": "ENEMY" }
    ] },
    "then": { "outcome": "LOSE" } },
  { "as": "NEXT", "when": { "test": "STUCK" }, "then": { "outcome": "DRAW" } }
]
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `as` | no | `MOVER` | Whose point of view the test is judged from, as `ALLY` and `ENEMY` are seen from "the mover" in a way. `MOVER`: the player who just played. `NEXT`: the player about to play. `ALLY`, `ENEMY`, `IN_ZONE` and a counter all read from that player. |
| `when` | yes | — | A [test](#tests). It names no place (`START`, `TARGET`, and `EACH` outside a `SOME`) and no `REMOVED` or `MOVED`: after a turn there is no way being worked out. |
| `then` | yes | — | `{ "outcome": ... }`, one of the outcomes below. |

| Outcome | When its check holds |
|---------|----------------------|
| `WIN` | The `as` player wins, and the game ends. |
| `LOSE` | The `as` player is out of the game: the turn order shrinks, and the others play on. When one player is left, the game ends and that player wins. In a two-player game it is the other player's win. |
| `DRAW` | The game ends with no winner. |
| `HIGHEST` | The game ends. The player with the highest [`score`](#score) wins, and a tie is a draw. Refused when the game has no `score`. |

A test sees the position as the turn left it, with the turn already handed on. `REPEATS` in an end
check therefore finds the position just reached already on record: threefold repetition is
`times: 3`. `STUCK` is the one test only an end check can use: the player to move has no way to
play. It must be judged `as` `NEXT`, since that is the player with no way.

**The draw by default.** When no end check holds and the player to move has no way to play, the
game is a draw. A game that wants such a player to lose writes a `STUCK` check with `LOSE` before
that point, and checkmate is one (see below).

An end check works out the ways of playing only when its test reads them (`STUCK`), and the ways
are then reused to start the turn, so a game with no `STUCK` check never works them out twice.

### `counters`

```json
"counters": ["CHECKS", "QUIET"]
```

The names of the numbers each player keeps. Every player holds one number for each name, starting
at 0, saved with the game (a game restored from before the counter existed finds it at 0). A
counter is changed by two effects, [`ADD` and `RESET`](#effects), usually in an
[`every_action`](#every_action) entry, and read by the `COUNTER` [test](#tests) and the `COUNTER`
[score term](#score). A name written anywhere without being declared here is refused.

Three-check counts the checks a player has given (`three_check.json`). Each move that leaves the
enemy king attacked by the mover adds one to the mover's `CHECKS`, and an end check reads it:

```json
{ "do": [
  { "if": { "test": "ATTACKED", "in": { "region": "BOARD", "where": { "owner": "ENEMY", "codes": ["KING"] } }, "by": "MOVER" },
    "then": [ { "effect": "ADD", "counter": "CHECKS" } ] }
] }
```

```json
{ "when": { "test": "COUNTER", "name": "CHECKS", "min": 3 }, "then": { "outcome": "WIN" } }
```

The fifty-move rule counts quiet turns in one counter that every player shares: a capture or a
pawn move resets it, any other turn adds one, and 100 of them end the game in a draw. The
`every_action` entry is

```json
{ "do": [
  { "if": { "test": "ANY", "of": [
      { "test": "REMOVED", "where": { "owner": "ENEMY" } },
      { "test": "MOVED", "where": { "codes": ["PAWN"] } } ] },
    "then": [ { "effect": "RESET", "counter": "QUIET", "to": "ALL" } ],
    "else": [ { "effect": "ADD", "counter": "QUIET", "to": "ALL" } ] }
] }
```

and the end check is `COUNTER QUIET min 100`, as in the list above.

### `score`

```json
"score": [
  { "term": "PIECES", "where": { "codes": ["STONE"] } },
  { "term": "TERRITORY", "where": { "codes": ["STONE"] } },
  { "term": "POINTS", "value": 7.5, "to": ["WHITE"] }
]
```

A list of terms, summed for each player. The sum is what the `HIGHEST` outcome compares, and the
session view shows it at any moment, as if the game ended now.

| Term | Written | Counts |
|------|---------|--------|
| `PIECES` | `{ "term": "PIECES", "where": F }` | The player's pieces on the board that match the [filter](#filters) `F` (every piece without `where`). `F` has no `owner`: the pieces are the player's. |
| `TERRITORY` | `{ "term": "TERRITORY", "where": F }` | The empty places of each orthogonally joined empty region whose neighbours, among the pieces that match `F`, belong to the player alone. A region touching nobody, or more than one player, counts for no one. Pieces that do not match `F` are ignored: they are not part of a region and make none touch a player. Only for two-dimensional boards. |
| `KEPT` | `{ "term": "KEPT", "zone": Z, "where": F }` | The pieces kept in the places of the player's part of the [zone](#zones) `Z` that match `F` (every piece without `where`): those standing on them and those held in a place that holds many (see [`KEPT`](#kept)). `F` has no `owner`: the places are the player's. |
| `COUNTER` | `{ "term": "COUNTER", "name": N }` | The player's counter. |
| `POINTS` | `{ "term": "POINTS", "value": 7.5, "to": ["WHITE"] }` | A fixed number, for the named players (Go's *komi*). Decimals are allowed, so `7.5` can never tie. |

`TERRITORY` is the one block only Go uses. It cannot be built from regions without a way to repeat
a rule for each region of a kind, which no other game asks for yet.

**There is no step for agreeing which pieces are dead.** The count is taken on the board as it is,
so a piece that is dead has to be captured in play before the players pass.

### What each old rule is written as

A game's end was once a set of named forms. Each is now an end check:

| Rule | As an end check |
|------|-----------------|
| Reach a zone with a piece (King of the Hill, Racing Kings) | `SOME in BOARD where ALLY codes [KING] that IN_ZONE at EACH`, as `MOVER`, `WIN` |
| Leave the opponent bare (Shatranj, Chaturanga) | `COUNT in BOARD where ENEMY except [exempt] max 0`, `WIN` |
| Join your zones with a chain (Hex) | `SOME in BOARD where ALLY codes [STONE] that ALL[ SOME in GROUP of EACH that IN_ZONE ... ]`, one inner `SOME` for each zone, `WIN` |
| Count the checks (Three-check) | an `ADD` in `every_action`, and `COUNTER CHECKS min 3`, `WIN` |
| Score after two passes (Go) | `PASSES in_a_row 2`, `HIGHEST`, with a `score` |
| Checkmate and stalemate | `as NEXT`, `ALL[STUCK, ATTACKED in BOARD where ALLY KING by ENEMY]`, `LOSE`; then `as NEXT`, `STUCK`, `DRAW` |
| Stalemate loses (Shogi, Xiangqi, Checkers) | `as NEXT`, `STUCK`, `LOSE` |
| Repetition | `REPEATS times n`, `DRAW` |
| Fifty moves | a `QUIET` counter, and `COUNTER QUIET min 100`, `DRAW` |
| Insufficient material | for `ALLY` and for `ENEMY`: `ALL[COUNT KING =1, COUNT except [KING, BISHOP, KNIGHT] max 0, COUNT [BISHOP, KNIGHT] max 1]`, `DRAW` |
| A king blown up (Atomic) | `as NEXT`, `COUNT in BOARD where ALLY KING max 0`, `LOSE` |

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
| `stages` | no | Named stages in which a player moves again before the turn is handed on, each `{ "name": N, "only": T }`. `only` is an optional [test](#tests). A way enters a stage with the `STAGE` effect. See [Stages](#stages). |

---

## `zones`

A zone is a named set of places, with one list for each player. Tests ask about zones with
`IN_ZONE` (see [Tests](#tests)), in a move, a drop or an end check.

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
| `name` | yes | The name tests use. Two zones may not share a name. |
| `places` | yes | For each player name, the places of that player's part of the zone; a name that is not a player of the game is refused. On a grid a place is `[x, y]` in **board coordinates** (absolute, not turned by `direction`) or a square's name such as `"4,1"`; on a map it is a place name. |

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
    "captured_as": null
  }
]
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `code` | yes | — | Unique identifier used everywhere else (starting positions, promotions, etc.). |
| `name` | no | — | Human-readable label. Not used by the engine. |
| `moves` | yes | — | The places the piece can go, as a list of move definitions (see below). A piece that never moves, such as a Go stone, has an empty list. |
| `captured_as` | no | `null` | The piece code that enters a bag when this piece is sent there by an effect's `removed_to`. `null` means the piece enters the bag as itself. Set this on promoted pieces to make them revert to their base form on capture — e.g. `"captured_as": "PAWN"` on a Shogi TOKIN or a Crazyhouse `P_QUEEN`. See the bags section for the full pattern. Replaces `demotes_to`. |
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
  "do": [
    { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } }, "where": { "owner": "ALLY" } } },
    { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "PALACE" },
    { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } }
  ]
}
```

A move's tests decide which moves are offered. They do not change what the piece attacks (see
[`threat_only`](#threat_only)), so a GENERAL kept in its palace still attacks squares outside it.

---

## Move definitions

Each entry in `moves` lists the places a piece can go by one pattern, and what the piece does
when it goes there.

```json
{
  "id": 0,
  "step": [0, 1],
  "do": [ ... ],
  "repeat": { ... }
}
```

| Field | Required | Description |
|-------|----------|-------------|
| `id` | yes | Integer unique within this piece. Errors name a move by it. |
| `step` | yes | `[dx, dy]` offset applied to the piece's position. Written from the neutral player's perspective; the engine rotates it per-player using the direction matrix. On a map it is a link label instead, such as `"next"` (see [Map boards](#map-boards)). |
| `players` | no | The players who may make this move. Empty (the default) means every player. |
| `threat_only` | no | When `true`, this move contributes to the piece's **attack map** but never produces legal moves. Default `false`. See [threat_only](#threat_only) below. |
| `do` | yes | The [steps](#steps) that run for each place the move reaches: tests that allow it, and the effects it has. |
| `repeat` | no | Makes the step repeat (for sliding pieces like rooks and bishops). |

`step` and `repeat` list the places the move can reach, the *candidates*. For each candidate, the
move's `do` runs with `START` the piece's square and `TARGET` the candidate. A test step that
fails means the piece cannot go to that candidate, so it is not offered. A candidate whose tests
all pass is offered, and the effects say what happens when it is picked.

Nothing is allowed unless a test says so. A move that must not land on a friend says so with a
test, and one that may capture needs no second move for it: not having an ally on `TARGET` allows
an empty square and an enemy alike. A move that does one thing on an empty square and another on
an enemy one is written as two moves with the same `step`, each with its own tests. The pawn's
diagonal is an example: one move captures, another is en passant.

When two moves of one piece reach the same `TARGET` and give the same answers, the first in the
list is kept.

In a move's `do` the only choice allowed is `CODE` (see [Choices](#choices)).

### `step` and direction

Steps are always written from the perspective of a player moving in the +y direction. The engine
transforms them at build time. Example: a forward step `[0, 1]` becomes `[0, -1]` for a player
with a 180° direction matrix (moving down the board).

---

### `threat_only`

When `threat_only: true`, the move is included in `calculate_threats` (the attack map used
for check detection) but **never** produces a legal move for the player.

This lets a piece project a threat through a move pattern it would never actually execute. The
canonical use case is Xiangqi's **flying-general rule**: the two generals must never face each
other on the same file with no piece between them. Rather than adding a game-specific flag, the
GENERAL gets one `threat_only` sliding move in the forward direction:

```json
{
  "id": 4,
  "step": [0, 1],
  "threat_only": true,
  "repeat": { "loop": true },
  "do": [
    { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } }, "where": { "owner": "ENEMY" } },
    { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } }
  ]
}
```

With direction matrices:
- RED's GENERAL (identity direction) projects this threat **up** the file toward BLACK's half.
- BLACK's GENERAL (180° direction) projects it **down** the file toward RED's half.

When the two generals are on the same file with nothing between them, each is attacked, because
the threat-only move reaches the other. The check test in Xiangqi's [`every_action`](#every_action)
then refuses every move that would leave them facing each other.

**Important**: the attack map does not judge a move's tests, with one exception. A move
threatens its `TARGET` unless a test before its first effect requires `TARGET` to be empty: a
`NOT` of a `COUNT` at `TARGET`, or a `COUNT` there with `"min": 0, "max": 0`, alone or in an `ALL`
at the top of the steps. Such a move can only go to an empty square, so it takes nothing: a pawn's
step forward threatens nothing, and its diagonal capture does. Every other test, and
`repeat.until`, only decides which moves are offered; for the attack map, a move follows its step
and ends at the first piece. So a GENERAL kept in its palace by `IN_ZONE` tests still projects the
`threat_only` sliding move across the entire file.

---

## `actions`

The top-level `actions` list is what a player may do on their turn. There is one kind of turn: an
**action** is made of the **choices** the player makes, the **tests** that allow it and the
**effects** it has. A piece move, a drop, a placing and a pass are all actions, written with the
same blocks.

```json
"actions": [
  { "name": "MOVE", "do": [
      { "choose": "PIECE", "where": { "owner": "ALLY" } },
      { "choose": "MOVE" }
  ] }
]
```

| Field | Required | Description |
|-------|----------|-------------|
| `name` | yes | Unique within the spec. It is shown in the history, and labels buttons. A player can answer with it (see below). |
| `players` | no | The players who may take it. Empty or absent means everyone. |
| `do` | yes | The steps, in order. An empty list is a pass. |

A spec has at least one action. The engine knows no action by name: `MOVE`, `DROP`, `STONE` and
`PASS` are only the names the catalogue uses. Chess has one action, `MOVE`. Shogi adds `DROP`. Go
has `STONE` and `PASS`.

### Steps

A step is exactly one of four things, told apart by its first key:

| Step | Written | Does |
|------|---------|------|
| choose | `{ "choose": KIND, ... }` | Asks the player (see [Choices](#choices)). |
| test | `{ "test": ... }` | A [test](#tests), judged on the position as the steps before it left it. If it fails, this way of taking the action does not exist. |
| effect | `{ "effect": KIND, ... }` | Changes the position (see [Effects](#effects)). |
| if | `{ "if": TEST, "then": [steps], "else": [steps] }` | Runs `then` when the test holds, otherwise `else`. Either list may be empty or absent. |

Steps run in order, and each sees the position as the effects before it left it. So a test after a
`MOVE` asks about the board after the piece has moved, and a choice after an effect (a promotion)
is asked about the piece where it now stands.

`START` and `TARGET` are the places choices set (see below), and `EACH` is set inside a
`FOR_EACH`. A step that names a place no earlier step sets is refused when the spec is read. After
an `if`, a place is set only when both of its branches set it.

An `if` is how a promotion reads: when the move ends in a zone, choose a code and set it.

```json
{ "if": { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "REACH_END" },
  "then": [
    { "choose": "CODE", "as": "promotion", "options": ["QUEEN", "ROOK", "BISHOP", "KNIGHT"] },
    { "effect": "SET_CODE", "at": { "place": "TARGET" }, "code": { "chosen": "promotion" } }
  ] }
```

### Choices

A choice asks the player one question. Each answer is a place, a code, a piece held in a bag or an
action's name.

| Choice | Written | Options | Sets |
|--------|---------|---------|------|
| `PIECE` | `{ "choose": "PIECE", "where": F }` | the places holding a piece that matches the [filter](#filters) `F` | `START` |
| `MOVE` | `{ "choose": "MOVE" }` | the places the piece at `START` reaches by its [moves](#move-definitions) | `TARGET`, then runs that move's own `do` |
| `PLACE` | `{ "choose": "PLACE", "in": R }` | the places of [region](#regions) `R` | `TARGET` |
| `HELD` | `{ "choose": "HELD", "in": B, "where": F }` | the codes held in the bag `B` (`{ "bag": "HAND", "of": "MOVER" }`) that match `F` (`codes` only; `where` may be left out) | `START`, as "this piece in the bag" |
| `CODE` | `{ "choose": "CODE", "as": NAME, "options": [codes] }` | the listed codes | the value `NAME` |

- **Only options that can finish are offered.** An option is offered only if some complete way of
  taking the action through it passes every test after it, including the steps of
  [`every_action`](#every_action) (check, liberties, repeated positions), and survives
  [`must`](#must). The player is never shown a dead end.
  A promotion to one piece needs no choice: write `SET_CODE` with that code.
- **`MOVE` needs a piece on the board.** It is refused when the spec is read unless an earlier
  `PIECE` choice sets `START`.
- **`HELD`** picks a piece held in a bag (see [Bags and the hand](#bags-and-the-hand)). `START` then
  names "the chosen piece in the bag": a `COUNT` at `START` sees that piece, so a filter with
  `codes` tells pieces apart, which is how Shogi's drop rules are written. An offset from `START`
  cannot be formed (a region that cannot be formed is false). A `HELD` filter takes `codes` only. The
  answer is `{ "held": CODE }` on the wire (`{ "hand": CODE }` is still read).
- **`CODE`** names its answer, and an effect uses it as `{ "chosen": NAME }`. Every choice takes
  an optional `as` name, but only `CODE` needs one.
- **In a piece's move, only `CODE`.** The other choices would set `START` or `TARGET` again.
- **No choice inside a `FOR_EACH`.**

The action every piece-moving game has is a `PIECE` choice and then a `MOVE` choice. A drop
chooses a `HELD` piece and then a `PLACE`. A placing, as in Go, chooses only a `PLACE`.

### Effects

An effect changes the position. A way of taking an action is the list of answers given and the
changes its effects make.

| Effect | Written | Does |
|--------|---------|------|
| `MOVE` | `{ "effect": "MOVE", "from": P, "to": P, "removed_to": R, "what": F, "count": N }` | The piece at `from` goes to `to`. Whatever stood on `to` is removed, as `REMOVE` does, and goes where `removed_to` says (optional; see [Bags and the hand](#bags-and-the-hand)). A `P` may also be a bag or a place that holds many, and then `what` and `count` say which pieces and how many move (see [Places that hold many, and Kalah](#places-that-hold-many-and-kalah)). With a `HELD` choice's `START` as `from`, one piece of that code leaves its bag and enters `to` as a new, unmoved piece. |
| `REMOVE` | `{ "effect": "REMOVE", "at": P, "removed_to": R, "what": F, "count": N }` | The piece at `P`, if any, leaves the board. With `removed_to` (`{ "bag": "HAND", "of": "MOVER" }`; the bag must be declared in `bags`), a piece of another player goes to that bag, as its `captured_as`; the mover's own piece is only removed. On a holder, `count` pieces that match `what` leave it, and all go to `removed_to`. |
| `PLACE` | `{ "effect": "PLACE", "at": P, "code": C, "count": N }` | A new, unmoved piece of code `C`, owned by the mover, is put on `P`. Whatever stood there is removed first. On a bag or a place that holds many, `count` pieces of `C` enter it. |
| `SET_OWNER` | `{ "effect": "SET_OWNER", "at": P }` | The piece at `P`, if any, now belongs to the mover. |
| `SET_CODE` | `{ "effect": "SET_CODE", "at": P, "code": C }` | The piece at `P`, if any, becomes code `C`. It keeps its owner and is otherwise new, with no moves counted and no flags. |
| `SET_FLAG` | `{ "effect": "SET_FLAG", "at": P, "flag": "EN_PASSANT", "turns": 1 }` | Sets a flag on the piece at `P`, if any. With `turns`, the flag is removed after that many moves, counting those of every player (`1` is one reply). Without it, the flag stays. A `COUNT` whose [filter](#filters) names the `flag` reads it. |
| `FOR_EACH` | `{ "effect": "FOR_EACH", "in": R, "do": [steps] }` | Runs `do` once for each place of region `R` (taken before any run starts), with `EACH` set to it. `do` may hold effects and `if`s, whose test may use `EACH`, but no choices and no bare tests. A `FOR_EACH` may hold another: inside it, `EACH` is the inner loop's place. |
| `STAGE` | `{ "effect": "STAGE", "name": N }` | When this way is played the turn is not handed on: the same player moves again, in the stage `N` of [`turns.stages`](#stages), if they have a way that stage allows. Where a way runs `STAGE` twice, the last counts. |
| `ADD` | `{ "effect": "ADD", "counter": N, "by": 1, "to": "MOVER" }` | Adds `by` (a whole number, default 1, never 0) to the mover's counter `N`, or to every player's with `"to": "ALL"`. `to` defaults to `MOVER`. See [`counters`](#counters). |
| `RESET` | `{ "effect": "RESET", "counter": N, "to": "ALL" }` | Sets counter `N` to 0, for the mover (the default) or for every player with `"to": "ALL"`. |

Values that effects take:

- **A place `P`:** `{ "place": "START" }`, `{ "place": "TARGET" }`, `{ "place": "EACH" }` inside
  a `FOR_EACH`, or `{ "place": "LAST" }`, with an optional `offset` (see [Places](#places)); or a bag, `{ "bag": "HAND", "of": "MOVER" }`.
- **A code `C`:** a piece code written as a string, or `{ "chosen": NAME }` for a `CODE` choice's
  answer. An unknown code, or a name no earlier `CODE` choice sets, is refused when the spec is
  read.
- **A bag (`R` of `removed_to`, `B` of `HELD`):** `{ "bag": NAME, "of": "MOVER" | "ENEMY" | <player name> }`; `of` is left out for a shared bag.
- **A filter `F` (`what`) and a count `N` (`count`):** see [Holders in effects](#holders-in-effects).
- **The owner:** the mover, except for pieces that come out of a holder (see [Holders in effects](#holders-in-effects)).

**Has moved.** A `MOVE` counts one more move for the piece it moves only when that piece is the
one a `PIECE` choice picked. No other effect counts moves. `SET_CODE` makes a new, unmoved piece,
as a promotion does. A castled rook is moved by an effect, so it stays unmoved. A piece dropped
from a bag, taken out of a holder, or put on the board by `PLACE`, enters unmoved, and a piece
moved into a bag or a holder forgets its moves and flags. A stone that copies itself (Ataxx)
leaves the original's count as it was. The `"moved"` filter reads these counts. So does the record
of positions that `REPEATS` looks at, but only in a game where some test, filter or flag reads
`moved` (castling, a double step): in any other game a piece's count is no part of the position, so
it cannot keep two positions apart that look the same.

### What a capture, a pass and a turn are

- A **capture** is a way that removes an enemy piece: by `REMOVE`, by `MOVE` onto it, or by `PLACE`
  onto it. Each removal is noted, so the captured material, the bags and the history's capture mark
  all come from the removals, never from an action's name, and the [`REMOVED`](#tests) test reads
  them. Three rules use it:
  - a forced capture, written `"must": [ { "test": "REMOVED", "where": { "owner": "ENEMY" } } ]`:
    when any piece can play a way that captures, only ways that capture are offered, for every
    piece (see [`must`](#must));
  - the history's capture mark;
  - a counter that counts the turns without a capture, such as the fifty-move rule's, with
    `REMOVED` in the test of an `if` that `RESET`s it (see [`counters`](#counters)).

  Keeping the turn after a capture is not a capture rule: a draughts jump does it with a
  [stage](#stages).
- A **pass** is a way that changes nothing, such as an action with an empty `do`. It is recorded in
  the history as a pass, and the `PASSES` test counts them.
- A **turn** is one way, or, with [stages](#stages), a chain of ways by the same player. A way
  that does not run a `STAGE` effect hands the turn on.

**Telling ways apart.** An action's name is always a valid answer. It narrows the ways to that
action's, and a player can give it at any point. When the answers so far match more than one
complete way and the ways come from different actions, the engine lists those actions and the player
picks one. This is how two placing actions, say `STONE` and `DISC`, are told apart: the player picks
the piece first or after clicking the point. An action that needs no more choices, such as a pass,
is played the moment it is named. Ways of one action that give the same answers keep the first in
the order of the spec.

**Cancelling.** Before any effect of the turn is showing, a player can cancel their answers and
start the turn again, as deselecting a piece does. Once an effect is showing, the answers cannot
be cancelled. A promotion is the example: the pawn already stands on the last row while the player
picks the piece, and the choice cannot be backed out of.

### Stages

A turn is one way. A **stage** lets a player move again before the turn is handed on, in a named
stage that limits which ways they may play. Two games use it: a draughts piece that keeps
jumping, and Kalah's free turn. (A double move, as in Monster Chess, is not a stage: it is a turn
order with a name repeated, `["WHITE", "BLACK", "BLACK"]`.)

A stage is declared under `turns`, and a way enters it with the `STAGE` effect:

```json
"turns": {
  "order": ["RED", "BLACK"],
  "stages": [
    { "name": "JUMPING",
      "only": { "test": "ALL", "of": [
        { "test": "SAME", "a": { "place": "START" }, "b": { "place": "LAST" } },
        { "test": "REMOVED", "where": { "owner": "ENEMY" } } ] } }
  ]
}
```

- A stage has a `name` and an optional `only`, a [test](#tests). Without `only` every way is
  allowed.
- `{ "effect": "STAGE", "name": N }` says: when this way is played, the turn is not handed on.
  The same player moves again, in stage `N`, if they have a way that passes its `only`. If they
  have none, the turn is handed on as usual. A way with no `STAGE` effect hands the turn on and
  leaves any stage. Where a way runs `STAGE` twice, the last counts.
- **`only`** is judged on each way the stage could play next, on the position that way leaves,
  with `START` the way's piece and `LAST` the place where the way before it ended. `TARGET` and
  `EACH` are refused there. A way that fails `only` is not offered. The ways that pass are then
  narrowed by [`must`](#must), as in any turn.
- **`LAST`** is a place: where the latest `MOVE` or `PLACE` of the turn put a piece (for a `MOVE`,
  its `to`). Inside a way it is the latest of the way so far, and before the way's own first
  `MOVE` or `PLACE`, in a stage, where the way before ended; in a stage's `only` it is where the
  way before ended. Where nothing has moved yet it names no place: an effect that uses it does
  nothing and a test of it fails, as `START` does with no piece chosen. It is allowed in steps
  and in a stage's `only`, and refused in [`must`](#must) and in end checks.
- **`SAME`**, `{ "test": "SAME", "a": P, "b": P }`, holds when both places are named and are the
  same place.
- When every way of a stage starts with the same piece, that piece is already chosen, and
  cancelling is refused: the jump lock, as players know it. A stage whose ways start in many places
  (Kalah's) leaves the choice open and may be cancelled.
- **End checks are judged when the turn is handed on**, not between the ways of a stage: a way
  that leaves a game decided but runs `STAGE` does not end it until the stage's last way.

**Draughts.** A jump that does not crown runs the stage, and the crowning jump does not, so the
turn ends there. Write the crown in the `then` of an `if` and the continuation in its `else`:

```json
{ "if": { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "REACH_END" },
  "then": [ { "effect": "SET_CODE", "at": { "place": "TARGET" }, "code": "KING" } ],
  "else": [ { "effect": "STAGE", "name": "JUMPING" } ] }
```

The king's jumps just end with the `STAGE` effect. In the stage `JUMPING` above, `only` says the
piece that moves is the one that landed (`SAME START LAST`) and that the way captures, so the
turn goes on only while the same piece has a capture left. The crowning jump is still a capture:
it is forced like any other, and it may end a multi-jump.

**Kalah.** The stores are a zone, `STORE`, and the stage `AGAIN` has no `only`. After the sowing,
a last stone that lands in the mover's own store gives another turn:

```json
{ "if": { "test": "IN_ZONE", "at": { "place": "LAST" }, "zone": "STORE" },
  "then": [ { "effect": "STAGE", "name": "AGAIN" } ] }
```

A player who has no stone to sow has no way, so the turn is handed on, and the end check
(`STUCK`) ends the game as before.

**Checked when the spec is read.** A stage name written twice; a `STAGE` effect that names no
stage; a stage `only` that uses a test an action's steps could not (the rules of `must`, with
`START` and `LAST` allowed, but no `PATH`: a stage has no step to fix its line); `LAST` in the
`until` of a repeat, where it names no place; and a `REPEATS` in a spec that has stages (a stage works the ways
out again in the middle of a turn, where "the turn handed on" is wrong). The old `jump_enabled`
switch and the `END_TURN` effect are refused too, naming `turns.stages` and `STAGE`.

---

## Tests

A rule's conditions are **tests**: small blocks that each hold or not, put together into a
tree. A test looks at a **region** of the board (one or more **places**) and counts the pieces
there that pass a **filter**, or asks whether those places are attacked, or whether a place is in
a zone.

```json
[
  { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START" } }, "where": { "moved": false } },
  { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START", "offset": [0, 1] } } } }
]
```

These are two test steps of the chess pawn's double step: the pawn on `START` has never moved, and
no piece stands on the square in front of it.

### Where tests appear

| Where | Holds when | `START` | `TARGET` |
|-------|------------|---------|----------|
| A test step of a piece's move | otherwise the move is not offered | the moving piece's square | the square the move lands on (for a slide, the square being judged) |
| A test step of an action | otherwise that way of taking the action does not exist | the square chosen by `PIECE`, or the piece held in a bag chosen by `HELD`; refused before either is chosen | the square chosen by `MOVE` or `PLACE`; refused before either is chosen |
| An `if` of a step | `then` runs when it holds, otherwise `else` | as in its steps | as in its steps |
| A repeat's `until` | one test; when it holds the slide goes no further | the moving piece's square | the square the slide has just reached |
| A test step of an [`every_action`](#every_action) entry | otherwise that way of taking the action does not exist | as the action's own steps set it; refused if one of the actions the entry applies to does not set it | as for `START` |
| An entry of [`must`](#must) | the ways that pass it are the only ones kept, if any pass | refused | refused |
| The `only` of a [stage](#stages) | the way is offered in that stage, otherwise it is not | the way's piece | refused |
| The `when` of an [end check](#end_checks) | the game ends, or a player leaves it, as `then` says | refused | refused |

Inside a `FOR_EACH`, `EACH` is the place being visited, in an `if` of the `do`.

### Tests

| Test | Written | True when |
|------|---------|-----------|
| `ALL` | `{ "test": "ALL", "of": [T, ...] }` | every test in `of` holds (an empty list holds) |
| `ANY` | `{ "test": "ANY", "of": [T, ...] }` | at least one test in `of` holds (an empty list does not) |
| `NOT` | `{ "test": "NOT", "of": T }` | `of` does not hold |
| `COUNT` | `{ "test": "COUNT", "in": R, "where": F, "min": 1, "max": n }` | the number of pieces in region `R` that pass filter `F` is at least `min` and at most `max`. `where` defaults to every piece, `min` to `1`, `max` to no limit |
| `ATTACKED` | `{ "test": "ATTACKED", "in": R, "by": "ENEMY", "keeping": F }` | some place in `R` is attacked, by the same attack map that check uses: by an opponent of the mover when `by` is `ENEMY` (the default), by the mover's own pieces when it is `MOVER`. A region with no places is not attacked. With `keeping`, a capture counts as a threat only if the attacker still has a piece that passes `F` after it, when it had one before (see below) |
| `IN_ZONE` | `{ "test": "IN_ZONE", "at": P, "zone": "PALACE" }` | place `P` is in the mover's part of the named zone (see [`zones`](#zones)) |
| `REMOVED` | `{ "test": "REMOVED", "where": F }` | the way has so far taken off the board a piece that passes filter `F`, whose `owner` is seen from the mover. A piece counts whether a `REMOVE`, a `MOVE` onto it or a `PLACE` onto it removed it, and whether or not it went to a bag. A piece of a player who is out of the game (no longer in the turn order, as in four-player chess once a player is checkmated) does not count: its pieces stay on the board, but taking one is no help against a check. `REMOVED` of an `ENEMY` piece is a [capture](#what-a-capture-a-pass-and-a-turn-are). Refused in an end check |
| `MOVED` | `{ "test": "MOVED", "where": F }` | the way has so far moved, along the board, a piece that passes filter `F`, as it stood when the turn began. Where `REMOVED` looks at what left the board, this looks at what travelled over it: a `MOVE` effect from one place to another. A piece put on the board from a bag or by `PLACE` has not moved. Refused in an end check |
| `REPEATS` | `{ "test": "REPEATS", "times": 3 }` | the position as the steps so far leave it, with the turn handed to the next player and timed flags ticked, is on record at least `times` times (`times` defaults to `1`: one earlier turn left it so, the ko rule). The starting position is not recorded. In an [end check](#end_checks) the position just reached is already on record, so a threefold repetition is `times: 3`. See [Repeated positions](#repeated-positions) |
| `SOME` | `{ "test": "SOME", "in": R, "that": T }` | at least one place of region `R` passes test `T`, which is judged with `EACH` that place. A `SOME` inside `T` makes `EACH` its own place instead. The region is taken before `T` runs |
| `SAME` | `{ "test": "SAME", "a": P, "b": P }` | both places are named and are the same place. Used in a [stage](#stages)'s `only`, to ask that the piece that moves is the one that landed (`START` and `LAST`) |
| `STUCK` | `{ "test": "STUCK" }` | the player to move has no way to play. Allowed only in an [end check](#end_checks) judged `as` `NEXT`; refused anywhere else when the spec is read |
| `PASSES` | `{ "test": "PASSES", "in_a_row": 2 }` | each of the last `in_a_row` turns, by anyone, was a pass: it changed nothing. `in_a_row` is at least 1 |
| `COUNTER` | `{ "test": "COUNTER", "name": N, "min": a, "max": b }` | the judging player's counter `N` is at least `min` and at most `max`. `min` defaults to `0`, `max` to no limit. In an end check the judging player is the one `as` names; in a way it is the mover, and the counter reads as it stood when the turn began (the turn's own `ADD`s are not seen). See [`counters`](#counters) |

Because `min` defaults to `1`, a `COUNT` with nothing else written means "at least one piece is
there". For "no piece is there", write `"min": 0, "max": 0`, or wrap the `COUNT` in a `NOT`.
`"max": 0` alone is refused: its `min` would be `1`, above its `max`. A `COUNT` whose filter is
`{ "empty": true }` counts the empty places of its region instead of pieces. A `COUNT` sees the pieces kept in a
place that holds many (see [Places that hold many, and Kalah](#places-that-hold-many-and-kalah))
as well as the pieces standing on places, and a place that keeps none is empty.

`REMOVED`, `MOVED` and `REPEATS` judge the way as it stands at their step, so they may be written
in any action's steps, but they matter most in [`every_action`](#every_action) and
[`must`](#must).

**`ATTACKED` with `keeping`.** In a game where some move takes more than its target (Atomic's
explosion), a capture can cost the attacker its own piece. With `"keeping": { "codes": ["KING"] }`
such a capture is no threat when it would remove the attacker's last king, so two kings may stand
side by side: neither can take the other without blowing up its own. The filter is about the
attacker's own pieces, so it takes `codes`, `except`, `flag` and `moved`, and refuses `owner` and
`empty`. It is only worked out in games where a move removes more than its target; elsewhere every
capture counts, as before. Atomic's check test is `ATTACKED ... keeping KING`, written where it
applies:

```json
{ "test": "ATTACKED", "in": { "region": "BOARD", "where": { "owner": "ALLY", "codes": ["KING"] } },
  "by": "ENEMY", "keeping": { "codes": ["KING"] } }
```

### Regions

| Region | Written | Places |
|--------|---------|--------|
| `AT` | `{ "region": "AT", "at": P }` | the one place `P` |
| `PATH` | `{ "region": "PATH", "from": P, "to": P, "ends": ["FROM", "TO"] }` | the places on the straight line between `from` and `to`, without its ends unless `ends` names them. `ends` defaults to none. Grids only |
| `AROUND` | `{ "region": "AROUND", "of": P, "through": "adjacent" }` | the neighbours of `P` through `adjacent` (the default) or `orthogonal`; on a map, `through` may also be one of the map's labels (see [Map boards](#map-boards)). `of` may be a region instead of a place: then it is the neighbours of the region's places that are not in it |
| `GROUP` | `{ "region": "GROUP", "of": P, "through": "orthogonal", "where": F }` | the places of the pieces joined to the piece at `P`: `P` itself, and every piece of the same owner that passes `F` and is a neighbour (through `through`, as in `AROUND`) of one already in the group. Empty when `P` holds no piece that passes `F`. `where` defaults to every piece |
| `LINE` | `{ "region": "LINE", "through": P, "along": [0, 1] }` | every square on the board's line through `P` in the direction `along`, both ways, `P` included. Disabled squares are skipped; the line stops only at the edge of the board. Grids only |
| `FOLLOW` | `{ "region": "FOLLOW", "from": P, "steps": N, "through": "next", "skipping": Z }` | the next `steps` places met by following the map label `through` (`next` by default) from `from`, going round when the links loop; the places of the zone `Z`, in the mover's part, are passed over without counting. Maps only. See [`FOLLOW`](#follow) |
| `BOARD` | `{ "region": "BOARD", "where": F }` | the places of the board, on a grid or a map, that pass filter `F` (as in `COUNT`); every place without `where`. Without it, it is what a `PLACE` choice picks from for a drop or a stone; with it, it finds pieces wherever they stand, such as the mover's king |

A straight line steps by -1, 0 or +1 on each axis. `LINE`'s `along` must be such a step (and not
`[0, 0]`); like an offset, it is written from the neutral player's view and turned for each
player.

`PATH` walks one such step from `from` until it reaches `to`. The line's direction is fixed when
the spec is read: it is the direction from `from` to `to`, counting `TARGET` as one step from
`START`. So the two ends may not be the same place, and that direction must be a straight line.
On a move that repeats, a `PATH` between `START` and `TARGET` takes no offset: its direction is
worked out from the first square and would be wrong past it. An action has no step to work the
direction out from, so a `PATH` is written only in a piece's move. Each of these is refused.

### Places

A place is `{ "place": "START" }`, `{ "place": "TARGET" }`, `{ "place": "LAST" }` or, inside a
`FOR_EACH`, `{ "place": "EACH" }` (see the table above for what each names, and
[Stages](#stages) for `LAST`: where the latest `MOVE` or `PLACE` of the turn put a piece), with an optional
`"offset": [dx, dy]`. The offset is written from the neutral player's
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
| `except` | none | its code is none of these, as in `["KING"]`. Written with `codes`, it narrows them; refused with `empty` |
| `flag` | — | it carries this state flag (see [`SET_FLAG`](#effects)) |
| `moved` | — | it has moved at least once (`true`), or never (`false`) |
| `empty` | `false` | `{ "empty": true }` takes a place that holds no piece (and keeps none, if it holds many). It is written alone, and only in a `COUNT`'s or a `BOARD`'s `where`, the filters of places; the filters of pieces (`PIECE`, `REMOVED`, `GROUP`) refuse it |

### A region that cannot be formed

A region cannot be formed when its place is off the board, or when a `PATH` crosses a hole or
the edge of the board (a `PATH` that is not a straight line is refused when the spec is read).
`COUNT` and `ATTACKED` over such a region are **false**, and `NOT` flips that as usual. So `NOT`
of a `COUNT` on a square off the board holds: a Xiangqi horse whose leg square would be off the
board is not blocked by it. `IN_ZONE` on a place off the board is false.

### Checked when the spec is read

A test that names something the game does not have is refused when the spec is read, never read
as false. The error names the problem and where it is, as in
``InvalidTest("an IN_ZONE in step 3 of move 0 of piece PADATI names the zone `REACH_ED`, which `zones` does not declare")``.

| Refused | Error |
|---------|-------|
| An unknown test, region or field, such as `"test": "CONT"` or an `owner` written beside `in` | `JsonError`, with `unknown variant` or `unknown field` |
| A place other than `START`, `TARGET`, `LAST` or `EACH` | `InvalidStep` in a step, `InvalidTest` in a repeat's `until` |
| `START`, `TARGET` or `EACH` where no earlier step sets it (`TARGET` before a `MOVE` or `PLACE` choice, `START` before a `PIECE` or `HELD` choice, `EACH` outside a `FOR_EACH`) | `InvalidStep` |
| `min` above `max` of a `COUNT` (including `"max": 0` with no `min`) or of a `COUNTER` | `InvalidTest` |
| An `owner` other than `ALLY`, `ENEMY`, `ANY`; a `by` other than `ENEMY`, `MOVER`; an end other than `FROM`, `TO` | `InvalidTest` |
| `empty` written with another field of a filter (`except` included), or in a filter of pieces (`PIECE`, `REMOVED`, `MOVED`, `GROUP`; a `HELD` filter is refused with `InvalidStep`) | `InvalidTest` |
| A `REPEATS` in a game with `turns.stages` (a stage works the ways out again in the middle of a turn, where the turn is not handed on) | `InvalidTest` |
| A `LAST` in a `must` entry or an end check | `InvalidStep` |
| `TARGET` or `EACH` in the `only` of a stage | `InvalidTest` |
| A stage name written twice; a `STAGE` effect naming a stage that `turns.stages` does not declare | `InvalidStep` |
| A code in `codes` or `except` that no piece has | `InvalidTest` |
| A zone that `zones` does not declare | `InvalidTest` |
| A `COUNTER` test, a counter effect or a `COUNTER` score term naming a counter that `counters` does not declare | `InvalidTest` in a test or a term, `InvalidStep` in an effect |
| A `STUCK` anywhere but an end check judged `as` `NEXT` | `InvalidTest` |
| A place name (`START`, `TARGET`, `EACH` outside a `SOME`), or a `REMOVED` or `MOVED`, in an end check | `InvalidStep` for a place, `InvalidTest` for the tests |
| A `PASSES` with `in_a_row` 0; a `REPEATS` with `times` 0 | `InvalidTest` |
| An `ATTACKED` with a `keeping` that has `owner` or `empty` | `InvalidTest` |
| A `HIGHEST` outcome in a game with no `score`; a `score` term whose filter has an `owner`; a `POINTS` term for a name that is not a player | `InvalidTest` |
| An unknown `outcome` or `as` of an end check | `JsonError`, with `unknown variant` |
| An `ADD` with `by` 0; a counter effect whose `to` is not `MOVER` or `ALL` | `InvalidStep` |
| A name declared twice (or empty) under `counters` | `InvalidStep` |
| The old keys `win_conditions`, `draw_conditions`, `leader` and `stalemate_loses` | `OldForm`, naming `end_checks` |
| The old keys `hand` and `hand_enabled` | `OldForm`, naming `bags` |
| The old key `jump_enabled` and the old effect `END_TURN` | `OldForm`, naming `turns.stages` and `STAGE` |
| A `KEPT` score term naming a zone `zones` does not declare, or with a filter that has an `owner` | `InvalidTest` |
| An `AROUND` or `GROUP` label the board does not have | `InvalidTest` on a grid, `UnknownLabel` on a map |
| A `LINE` along `[0, 0]`, or along a step bigger than 1 on an axis | `InvalidTest` |
| A `PATH` whose ends are the same place; a `PATH` whose line is not straight (as from `START` to `START` + `[2, 1]`, or from `START` to `TARGET` on a step of `[1, 2]`); on a move with `repeat`, a `PATH` between `START` and `TARGET` with an offset on either end | `InvalidTest` |
| A `PATH` in an action | `InvalidStep` |
| An offset, `PATH` or `LINE` on a map | `NotOnAMap` |
| An offset or `along` with the wrong number of entries, or too large | `MismatchedDimensions`, `ValueOutOfRange` |
| Two zones that share a name | `DuplicateZone` |
| A zone whose `places` lists a name that is not a player of the game | `UnknownPlayerInZone` |

Actions and steps are checked the same way, each error naming the step (`step 2 of action DROP`, `step 1 of move 0 of piece MARCHER`):

| Refused | Error |
|---------|-------|
| A step with none, or more than one, of `choose`, `test`, `effect` and `if` | `JsonError` |
| An unknown choice or effect, an unknown field of any step, a spec with no `actions`, a move with no `do` | `JsonError`, with `unknown variant`, `unknown field` or `missing field` |
| A `MOVE` choice with no earlier `PIECE` choice | `InvalidStep` |
| A `{ "chosen": NAME }` that no earlier `CODE` choice names | `InvalidStep` |
| A piece code that no piece has, in a `PLACE` or `SET_CODE` effect or in a `CODE` choice's `options` | `UnknownPieceInRule` |
| A choice inside a `FOR_EACH`; a test written bare in a `FOR_EACH` (put it in an `if`); a choice other than `CODE` in a piece's move | `InvalidStep` |
| A `HELD` filter with an `owner`, `flag`, `moved` or `empty` | `InvalidStep` |
| A bag whose `name` is empty, has an `@`, or is used by another bag or by a place of the map; a `capacity` of 0 on a bag or a map place; a `start` with a count of 0, above the `capacity`, or on a map place of capacity 1 | `InvalidStep` |
| A `start` that names a piece code the game does not have | `UnknownPieceInRule` |
| A bag with an unknown `for` or field; a `removed_to` or bag written in a form that is not `{ "bag": ..., "of": ... }` (the old `{ "hand": "MOVER" }` among them); a `count` that is not a whole number above 0, `"ALL"` or `{ "count": ... }` | `JsonError` |
| A `removed_to`, a `HELD` choice or a place that names a bag `bags` does not declare; that leaves out `of` for a bag each player has, writes one for a shared bag, or names an `of` that is not `MOVER`, `ENEMY` (two players only) or a player; a place with a bag and also a `place` or an `offset`, or an `of` and no bag | `InvalidStep` (`InvalidTest` in a test) |
| A `what` or a `count` in a game with no bag and no place that holds many; a `what` with a `flag`, `moved` or `empty`; a counted `where` with `empty`; a `FOLLOW` whose `steps` is `"ALL"`; a `FOLLOW` that skips a zone the game does not have, or follows a label that leads to several places from one place | `InvalidStep` |
| A `FOLLOW` on a grid | `NotOnAMap` |
| A one-way link whose ends are the same point, or are not points of the map | `InvalidMap` |
| An action name used twice; an action for a name that is not a player of the game; a spec with an empty `actions` | `InvalidStep` |
| An `every_action` entry with an empty `for`, or a `for` naming an action the game does not have; a choice in an `every_action` entry, even inside an `if` or a `FOR_EACH` | `InvalidStep` |
| An `every_action` step that names `START`, `TARGET` or `EACH` where one of its actions does not set it (the error names the entry and the action); a `must` entry that names a place | `InvalidStep` |
| A `must` entry that is not a test | `JsonError` |

### Examples

The Xiangqi horse may not leap if a piece stands on its first step, its "leg":

```json
{
  "id": 4,
  "step": [1, 2],
  "do": [
    { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } }, "where": { "owner": "ALLY" } } },
    { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START", "offset": [0, 1] } } } },
    { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } }
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
  "repeat": { "until": { "test": "ANY", "of": [] }, "loop": true },
  "do": [
    { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } }, "where": { "owner": "ENEMY" } },
    { "test": "COUNT", "in": { "region": "PATH", "from": { "place": "START" }, "to": { "place": "TARGET" } }, "min": 1, "max": 1 },
    { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } }
  ]
}
```

In Solidarity chess a piece moves only with a friend beside it:

```json
{ "test": "COUNT", "in": { "region": "AROUND", "of": { "place": "START" } }, "where": { "owner": "ALLY" }, "min": 1 }
```

Shogi's *nifu*: no pawn may be dropped on a file that already holds one of your unpromoted pawns.
It is a test step of the drop action, after the `HELD` and `PLACE` choices:

```json
{ "test": "NOT", "of": { "test": "COUNT", "in": { "region": "LINE", "through": { "place": "TARGET" }, "along": [0, 1] }, "where": { "owner": "ALLY", "codes": ["PAWN"] } } }
```

---

## Rules that apply to every move

Some rules look at every move, whatever the piece or the action: a king may not be left in check,
a stone with no liberty is taken, a capture is forced. Two top-level lists write them once.

### `every_action`

```json
"every_action": [
  { "for": ["STONE"], "do": [ ...steps... ] },
  { "do": [ ...steps... ] }
]
```

| Field | Required | Description |
|-------|----------|-------------|
| `for` | no | The names of the actions the entry applies to. Absent means every action, a pass included. An empty list, or a name that no action has, is refused. |
| `do` | yes | [Steps](#steps): tests, effects and `if`s. A choice is refused, even inside an `if` or a `FOR_EACH`. |

The steps of each entry that applies run after the action's own steps, entries in the order
written, as if they were written at the end of the action. They see the position as the action
left it, with its `START` and `TARGET`. A test that fails removes that way of taking the action,
as a test inside the action would, and an effect is part of the way, as the action's own are.

**Check.** Every chess-like game in the catalogue has this entry (`chess.json`):

```json
"every_action": [
  { "do": [
    { "test": "NOT", "of": { "test": "REMOVED", "where": { "owner": "ALLY", "codes": ["KING"] } } },
    { "test": "ANY", "of": [
      { "test": "NOT", "of": { "test": "ATTACKED",
          "in": { "region": "BOARD", "where": { "owner": "ALLY", "codes": ["KING"] } }, "by": "ENEMY" } },
      { "test": "REMOVED", "where": { "owner": "ENEMY", "codes": ["KING"] } }
    ] }
  ] }
]
```

The first test says the way did not take the mover's own king off the board, which is Atomic's
"a capture may not blow up your own king". The second says no enemy attacks a square holding the
mover's king, unless the way took an enemy king. `BOARD` with `where` finds the kings wherever
they stand. With no king of the mover's on the board neither test can fail, and with more than
two players taking any enemy king is enough. A pinned piece, a king stepping into an attack and a
drop that leaves the king attacked are all refused by this one entry.

**No check on the opponent.** Racing Kings, where giving check is forbidden, adds a third test to
the entry:

```json
{ "test": "NOT", "of": { "test": "ATTACKED",
    "in": { "region": "BOARD", "where": { "owner": "ENEMY", "codes": ["KING"] } }, "by": "MOVER" } }
```

**Checkmate is an end check.** The entry above only refuses moves. That a player with no way to
play and an attacked king has lost is the first of the two `STUCK` checks in
[`end_checks`](#end_checks), and the stalemate draw is the second. Atomic's two kings may stand
side by side because its check tests read `ATTACKED ... keeping KING` (see [Tests](#tests)).

Go's liberty capture, its suicide rule and ko are an entry for the action that places a stone
(see [Liberty capture](#liberty-capture)).

### `must`

```json
"must": [ { "test": "REMOVED", "where": { "owner": "ENEMY" } } ]
```

A list of [tests](#tests), each an obligation. They are judged on the ways of every action that
passed all their steps. Each entry narrows those ways, in the order written: if some way passes
the test, only the ways that pass stay; if none passes, nothing changes, and the next entry looks
at the same ways. An entry is judged on the way as it ended, so it names no place: `START`,
`TARGET` and `EACH` are refused.

The entry above is Checkers' forced capture: when some way captures, only the ways that capture
are offered, for every piece. It applies to every action, so a game with a forced capture also
offers no drop and no pass while a capture exists. Leave it out and capturing is a free choice.

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
| `until` | no | the next square is occupied | A [test](#tests), judged on each square the slide reaches, with `TARGET` that square and `START` the moving piece's square. The square is still judged by the move's `do` (so an enemy there can be captured); when `until` holds, the slide goes no further. The default is the `COUNT` above: stop at the first piece. |
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

## Coordinate conventions

- All `[dx, dy]` values in `step`, and every `offset` and `along` (in a test, or on an effect's
  place), are written in **neutral coordinates** (as if the player moves in the +y direction).
- The engine rotates them per-player using the direction matrix at build time.
- Absolute positions in `starting_positions` and in the `places` of `zones` are written in
  **board coordinates** (origin at bottom-left).

---

## Complete annotated example — en passant

White's en passant capture to the left, from `chess.json` (the promotion step that every pawn
move ends with is left out here):

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

Walk-through:

1. The pawn tries to step diagonally forward-left `[-1, 1]`.
2. The first test asks for an **empty** target square. An enemy there is an ordinary capture, and
   another move of the pawn (with the same step) does that.
3. The second test counts the pieces on the square beside the pawn, at `[-1, 0]`, that carry the
   `EN_PASSANT` flag. It holds when there is at least one. If it does not, the move is not offered.
4. The `REMOVE` takes the pawn beside it. That removal is a capture, so it counts as one.
5. The `MOVE` puts the pawn on the empty square.

The `EN_PASSANT` flag is set by the pawn's double step, with the last step of its `do`:

```json
{ "effect": "SET_FLAG", "at": { "place": "TARGET" }, "flag": "EN_PASSANT", "turns": 1 }
```

---

## Complete annotated example — kingside castling

White's kingside castle, from `chess.json`:

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

Walk-through:

1. The king steps two squares right `[2, 0]`. Only White has this move (`players`): Black's rook
   stands at a different distance, so Black has moves of its own.
2. Guards, each a test step:
   - `NOT` `COUNT` at `TARGET` — the landing square is empty.
   - `COUNT` at `START` where `moved` is `false` — the king has never moved.
   - `NOT` `ATTACKED` at `START` — the king is not in check.
   - `COUNT` in the `PATH` to `[3, 0]`, `min` and `max` `0` — the squares between king and rook
     are empty.
   - `NOT` `ATTACKED` at `TARGET` — the landing square is safe.
   - `NOT` `ATTACKED` in the `PATH` from `START` to `TARGET`, with its `TO` end — every square
     the king crosses or lands on is safe.
   - Two `COUNT`s at `[3, 0]` — an allied rook stands there, and it has never moved.
3. The first `MOVE` moves the king. It is the piece the `PIECE` choice picked, so it counts a move.
4. The second `MOVE` moves the rook next to the king. It is not the chosen piece, so the rook
   stays unmoved.

If the rook square were off the board, both `COUNT`s on it would be false and the move would not
be offered.

---

## Bags and the hand

A **bag** is a store of pieces off the board. It keeps counts of pieces, per owner and code, not a
pile with an order. The **hand** is not a separate thing: it is a bag called `HAND`, one for each
player, and it is the mechanism behind Shogi and Crazyhouse drops. (A **place** is on the board; a
bag is not. A map place that holds many pieces is a place, see
[Places that hold many, and Kalah](#places-that-hold-many-and-kalah).)

### Declaring bags

A game has the bags its top-level `bags` list declares, and no others. A game with no `bags` has no
hand: removed pieces leave the game, and nothing enters a bag by itself.

```json
"bags": [
  { "name": "HAND", "for": "EACH_PLAYER" },
  { "name": "SUPPLY", "capacity": 19, "start": { "WOOD": 19 } }
]
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `name` | yes | — | The name effects and choices use. Not empty, no `@`, and not the name of another bag or of a place of the map. |
| `for` | no | one shared bag | `"EACH_PLAYER"` gives every player a bag of this name (the hand is one). Without it there is a single bag that belongs to nobody. |
| `capacity` | no | no limit | The most pieces it holds, at least 1. |
| `start` | no | empty | What it holds when the game starts, as `{ "CODE": n }`: codes the game has, counts above 0, no more than `capacity` in all. A bag each player has starts with this for each player; the pieces of a shared bag are nobody's. |

The top-level `hand` (`{ "enabled": ... }`) and `hand_enabled` are refused, with an `OldForm` error
that names `bags`.

A bag is named where an effect or a choice needs one as `{ "bag": "HAND", "of": "MOVER" }`. For a
bag each player has, `of` is written and says whose: `MOVER`, `ENEMY` (in a game of two players) or
a player's name. For a shared bag `of` is left out. A bag the game does not declare, a missing or an
extra `of`, and an `of` that names nobody are refused when the spec is read.

### What goes into a bag

An effect that removes a piece sends it to a bag when it carries `removed_to`, always written
`{ "bag": NAME, "of": WHO }`, on a `MOVE` (the piece on its `to`) or a `REMOVE`:

```json
{ "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" },
  "removed_to": { "bag": "HAND", "of": "MOVER" } }
```

Only a piece of another player enters the bag; the mover's own piece is only removed. The piece that
enters is its **captured form**:

- If the removed piece has `"captured_as": null` (or the field is absent), the piece code itself
  enters the bag. A captured ROOK becomes a ROOK in the hand.
- If the removed piece has `"captured_as": "PAWN"`, a PAWN enters the bag instead. This is how
  promoted pieces revert to their base form — in Shogi a captured DRAGON becomes a ROOK in hand;
  in Crazyhouse a captured `P_QUEEN` becomes a PAWN in hand.

A piece that enters a bag becomes a count: it forgets its flags and its moves. Whose it is
afterwards depends on the bag:

| The piece enters | It belongs to |
|------------------|---------------|
| a bag each player has | the player whose bag it is: a rook the mover takes is the mover's, in the mover's hand |
| a shared bag | nobody |

A bag that is full (its `capacity`) takes nothing more: a way whose removal would pass it is not
offered. The bags are saved with the rest of the game state. A game saved before bags, with a
`hand`, loads into the bags `HAND`.

A piece can also be put on the board with no bag at all, by a `PLACE` effect: see
[Placing a piece](#placing-a-piece). Bags are also moved into and out of by `MOVE`, `REMOVE` and
`PLACE` themselves: see [Places that hold many, and Kalah](#places-that-hold-many-and-kalah).

### The captured-form pattern

A key design decision is how to distinguish pieces that change form on capture from pieces of
the same movement type that should not.

**Shogi**: every promoted piece is a distinct code (DRAGON, HORSE, TOKIN, …) with
`"captured_as"` pointing to its base. The base pieces (ROOK, BISHOP, PAWN, …) have no
`"captured_as"` and enter the hand as themselves.

**Crazyhouse**: this game requires that promoted pawns revert to pawns, but native queens do not
revert to anything. The solution is the same pattern: give promoted pawns their own codes
(`P_QUEEN`, `P_ROOK`, `P_BISHOP`, `P_KNIGHT`) with `"captured_as": "PAWN"`, and let native
pieces (QUEEN, ROOK, etc.) carry no `"captured_as"`. The pawn's promotion then offers the P_ codes
rather than the native ones:

```json
{ "if": { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "REACH_END" },
  "then": [
    { "choose": "CODE", "as": "promotion", "options": ["P_QUEEN", "P_ROOK", "P_BISHOP", "P_KNIGHT"] },
    { "effect": "SET_CODE", "at": { "place": "TARGET" }, "code": { "chosen": "promotion" } }
  ] }
```

```json
{
  "code": "P_QUEEN",
  "name": "promoted pawn — queen",
  "captured_as": "PAWN",
  "moves": [ /* identical to QUEEN */ ]
}
```

A native QUEEN captured from its starting square enters the hand as QUEEN. A `P_QUEEN` captured
after promotion enters the hand as PAWN. The engine has no special knowledge of either game's
rules — it just follows `captured_as`.

### Dropping

A drop is an action: choose a piece held in the hand, choose a place, test the place, and `MOVE`
the piece from the hand to it. This is Crazyhouse's (`crazyhouse.json` also writes `removed_to` on
the `MOVE`, where it does nothing, since the square must be empty):

```json
{ "name": "DROP", "do": [
  { "choose": "HELD", "in": { "bag": "HAND", "of": "MOVER" } },
  { "choose": "PLACE", "in": { "region": "BOARD" } },
  { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } } } },
  { "test": "ANY", "of": [
    { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START" } }, "where": { "codes": ["PAWN"] } } },
    { "test": "NOT", "of": { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "TERMINAL_RANK" } }
  ] },
  { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } }
] }
```

1. The player picks a piece held in the hand (`HELD`). Each code in the bag is offered once. In a
   live match this answer goes to the server, as choosing a piece on the board does: it is written
   `{ "held": CODE }` (`{ "hand": CODE }`, from before bags, is still read).
2. The player picks a place of the board (`PLACE` in `BOARD`). A place is offered only when
   some complete way through it passes every test after it: the place is empty, the piece's own
   rules hold (below), and the steps of `every_action` hold after it (in Crazyhouse and Shogi,
   the drop does not leave the player's king in check).
3. The `MOVE` takes the piece out of the hand and puts a **fresh** piece on the place, with no
   moves counted and no state flags. It belongs to the player whose hand it was in, or to the
   mover if it was nobody's. The turn advances.

The freshness of dropped pieces has implications: a dropped piece has never moved, so it passes
every `"moved": false` test. A dropped rook can take part in castling. A dropped pawn may
double-step from any rank it was dropped on, not only its second: the double step only asks that
the pawn has not moved and that the square ahead is empty. Standard Crazyhouse allows the double
step only from the second rank (see [Known limitations](#known-limitations)).

### Drop rules

A drop's rules are test steps after its choices. `START` is the piece held in the bag and `TARGET`
is the place picked. A `COUNT` at `START` sees that piece, so its `codes` say which piece it is. To
give each kind of piece its own rule, put the rule in an `ANY` whose first test says "this is not
that piece", as the pawn's rule above does.

Two kinds are common:

**Not in a zone** — `NOT` of an `IN_ZONE` keeps drops off a set of squares. Use a zone with
identical lists for all players when the squares are the same for everyone. Crazyhouse uses
`TERMINAL_RANK` to keep pawn drops off rows 0 and 7 for both players; Shogi uses `PROMO_FORCED`
(different for each player) to keep drops off squares where the piece could never move again.

**Not on a file with a pawn** — `NOT` of a `COUNT` over the `LINE` through `TARGET` along
`[0, 1]`, the file. This is Shogi's *nifu* rule (no two unpromoted pawns on the same file).

```json
// Shogi's DROP, the rule for a pawn: not in the last row, and nifu
{ "test": "ANY", "of": [
  { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START" } }, "where": { "codes": ["PAWN"] } } },
  { "test": "ALL", "of": [
    { "test": "NOT", "of": { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "PROMO_FORCED" } },
    { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "LINE", "through": { "place": "TARGET" }, "along": [0, 1] }, "where": { "owner": "ALLY", "codes": ["PAWN"] } } }
  ] }
] }
```

Shogi's `DROP` has one such rule each for the pawn, the lance and the knight, and `shogi.json`
declares the same bag as Crazyhouse: `"bags": [{ "name": "HAND", "for": "EACH_PLAYER" }]`.

---

## Complete annotated example — Crazyhouse hand

Crazyhouse is standard chess with one rule addition: captured pieces enter the capturer's hand
and can be dropped back as that player's own piece. Promoted pawns revert to pawns on capture.

**Top-level additions over chess:**

```json
"bags": [{ "name": "HAND", "for": "EACH_PLAYER" }],
"end_checks": [
  { "when": { "test": "REPEATS", "times": 3 }, "then": { "outcome": "DRAW" } },
  ...checkmate and stalemate, as in chess...
]
```

Each `MOVE` that takes a piece, and each en passant `REMOVE`, also carries
`"removed_to": { "bag": "HAND", "of": "MOVER" }`, so the piece it takes enters the capturer's hand.

The fifty-move and insufficient-material checks are omitted — in Crazyhouse the hand makes material
counts dynamic and the fifty-move rule rarely applies.

**A second action, `DROP`** (shown above), beside chess's `MOVE`:

```json
"actions": [
  { "name": "MOVE", "do": [ { "choose": "PIECE", "where": { "owner": "ALLY" } }, { "choose": "MOVE" } ] },
  { "name": "DROP", "do": [ ... ] }
]
```

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

**Pawn — a promotion that offers the P_ codes:**

```json
{
  "id": 0,
  "step": [0, 1],
  "do": [
    { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } } } },
    { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } },
    { "if": { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "REACH_END" },
      "then": [
        { "choose": "CODE", "as": "promotion", "options": ["P_QUEEN", "P_ROOK", "P_BISHOP", "P_KNIGHT"] },
        { "effect": "SET_CODE", "at": { "place": "TARGET" }, "code": { "chosen": "promotion" } }
      ] }
  ]
}
```

The promotion options point to P_ codes rather than native codes.

**Promoted-pawn pieces:**

```json
{ "code": "P_QUEEN",  "name": "promoted pawn — queen",  "captured_as": "PAWN", "moves": [ /* queen moves */ ] },
{ "code": "P_ROOK",   "name": "promoted pawn — rook",   "captured_as": "PAWN", "moves": [ /* rook moves  */ ] },
{ "code": "P_BISHOP", "name": "promoted pawn — bishop", "captured_as": "PAWN", "moves": [ /* bishop moves*/ ] },
{ "code": "P_KNIGHT", "name": "promoted pawn — knight", "captured_as": "PAWN", "moves": [ /* knight moves*/ ] }
```

**Native pieces** (QUEEN, ROOK, BISHOP, KNIGHT) carry no `captured_as` and enter the hand as
themselves when captured.

**Walk-through of a capture-demotion cycle:**

1. WHITE pawn reaches row 7, promotes to `P_QUEEN`. Board now has a `P_QUEEN` belonging to WHITE.
2. BLACK captures the `P_QUEEN`. Because `captured_as: "PAWN"`, a PAWN enters BLACK's hand.
3. BLACK drops the PAWN on any empty square not on rows 0 or 7 (the `TERMINAL_RANK` test of `DROP`).
4. The dropped pawn is a fresh piece: no state flags, no moves counted, owned by BLACK.

If instead BLACK had captured WHITE's native QUEEN (from starting position, no `captured_as`),
a QUEEN would have entered BLACK's hand — and BLACK could drop a QUEEN anywhere empty.

---

## Places that hold many, and Kalah

On a grid a square holds one piece. A map place can hold many: a pit of stones, a clearing full of
warriors, a bank. Such a place is a **holder**, like a bag, and the blocks that speak of a place
speak of a holder too. Nothing in the engine is written for one game: Kalah (Mancala) is played by
the blocks below.

### Capacity and start

Two more fields of a [map](#map-boards) place:

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `capacity` | no | `1` | How many pieces the place holds, at least 1. Above 1 the place is a holder: it keeps counts of pieces and never a piece of its own. With `1`, or without it, the place holds one piece as before. |
| `start` | no | empty | What the holder has when the game starts, as `{ "STONE": 4 }`: only with a `capacity` above 1, codes the game has, counts above 0, no more than `capacity` in all. The pieces are nobody's. |

Grid squares always hold one piece. A game that needs a place of many writes a map.

### Holders in effects

`MOVE`, `REMOVE` and `PLACE` take a bag (`{ "bag": NAME, "of": WHO }`, as in `removed_to`) or a
holder place (`START`, `TARGET` or `EACH`, set to one) wherever they take a place. They also take
two optional fields, which say which pieces and how many a holder gives or takes:

| Field | On | Description |
|-------|----|-------------|
| `what` | `MOVE`, `REMOVE` | A [filter](#filters) of the pieces taken out of a holder, with `codes`, `except` and `owner` only (a piece in a holder has no flags, and "has moved" and "empty" mean nothing there). `owner` is seen from the mover; a piece that belongs to nobody is not `ALLY`. Every piece when not written. |
| `count` | `MOVE`, `REMOVE`, `PLACE` | How many: a whole number above 0, `"ALL"` (everything that matches; on a `PLACE` it leaves no way, so do not write it), or `{ "count": { "in": R, "where": F } }`, the number of pieces of [region](#regions) `R` that pass filter `F`, counted as a `COUNT` test counts, in the position the steps so far leave. One when not written. |

`what` and `count` act where a holder is involved: they say what leaves a holder, or how many
enter one. A game with no bag and no place that holds many refuses them. What a `MOVE` does depends
on its two ends:

| `from` → `to` | Does |
|---------------|------|
| place → place | As before: the piece moves, and what stood on `to` is removed. |
| place → holder | The piece becomes a count in the holder, forgetting its flags and moves. |
| holder → place | `count` must be 1 (the default). A new, unmoved piece of that code stands on the place, and what stood there is removed. It belongs to the owner it had in the holder, and a nobody's piece becomes the mover's. |
| holder → holder | `count` of the pieces that match `what` move across, taken owner by owner in the order they arrived. |

A `REMOVE` of a holder takes `count` pieces that match `what` out of it, all going to `removed_to`
if it is written. A `PLACE` on a holder puts `count` new pieces of the code in it, by the owner
rule below.

**A way that cannot be made is not offered**, as when a test fails: taking more than the holder has,
or putting more than its capacity allows, removes that way, and no half of it is done.

**Whose the pieces are when they enter a holder:** in a bag each player has, that player's (the
bag's); in a shared bag, nobody's; in a map place that holds many, the owner they had.

Counting reads holders too. A `COUNT` sees the pieces kept in a holder place as well as the pieces
standing on places, a place that keeps none counts as empty (`{ "empty": true }`), and `BOARD`'s
`where` reads the same way.

### `FOLLOW`

```json
{ "region": "FOLLOW", "from": { "place": "TARGET" }, "steps": 3, "through": "next", "skipping": "OTHER_STORE" }
```

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `from` | yes | — | The place to start from. The walk begins at the place after it. |
| `steps` | yes | — | How many places to take: a whole number above 0 or a `{ "count": ... }` as above (a count may come to 0, and then the region is empty). Not `ALL`. |
| `through` | no | `next` | The map label to follow (see [Map boards](#map-boards)). |
| `skipping` | no | none | A zone. Its places, in the mover's part of it, are walked over and do not count as steps. |

`FOLLOW` is the next `steps` places met by following `through` from `from`, in order, and going
round when the links loop. It is for maps only (a grid is refused, with `NotOnAMap`). The label
must lead to one place from every place it leaves; a label that leads to several places from one is
refused when the spec is read. The walk is
capped, so a ring whose places are all skipped ends instead of going on for ever. Inside a
`FOR_EACH` it is how a sowing is written: one stone to each place of the walk.

### One-way links

A written link `{ "from": "a", "to": "b" }` goes one way, under `next`, beside the two-way
`["a", "b"]` (see [Map boards](#map-boards)). A ring of one-way links gives each place one `next`,
which is what `FOLLOW` needs.

### `KEPT`

The [score](#score) term `{ "term": "KEPT", "zone": Z, "where": F }` counts, for each player, the
pieces kept in the places of the player's part of zone `Z`: those standing on them and those held
in a holder there. It is how a mancala score is written.

### Complete annotated example — Kalah

Kalah (Mancala), `crates/engine/src/tests/kalah/spec.json` (a test of the
engine, not a catalogue game). Two rows of six pits and a store at each end make a ring of 14
places. Each pit starts with four stones. A turn takes the stones of one of your pits and sows them,
one to each place that follows, going round, over your own store and past the other player's. A
last stone that lands in your own store gives you another turn.

The board: pits hold up to 48 (all the stones there are), the stores start empty, and one-way links
make the ring `a1 … a6, sa, b1 … b6, sb`, and back to `a1`:

```json
"board": {
  "shape": "map",
  "places": [
    { "name": "a1", "at": [1, 0], "capacity": 48, "start": { "STONE": 4 } },
    { "name": "a2", "at": [2, 0], "capacity": 48, "start": { "STONE": 4 } },
    { "name": "a3", "at": [3, 0], "capacity": 48, "start": { "STONE": 4 } },
    { "name": "a4", "at": [4, 0], "capacity": 48, "start": { "STONE": 4 } },
    { "name": "a5", "at": [5, 0], "capacity": 48, "start": { "STONE": 4 } },
    { "name": "a6", "at": [6, 0], "capacity": 48, "start": { "STONE": 4 } },
    { "name": "sa", "at": [7, 1], "capacity": 48 },
    { "name": "b1", "at": [6, 2], "capacity": 48, "start": { "STONE": 4 } },
    { "name": "b2", "at": [5, 2], "capacity": 48, "start": { "STONE": 4 } },
    { "name": "b3", "at": [4, 2], "capacity": 48, "start": { "STONE": 4 } },
    { "name": "b4", "at": [3, 2], "capacity": 48, "start": { "STONE": 4 } },
    { "name": "b5", "at": [2, 2], "capacity": 48, "start": { "STONE": 4 } },
    { "name": "b6", "at": [1, 2], "capacity": 48, "start": { "STONE": 4 } },
    { "name": "sb", "at": [0, 1], "capacity": 48 }
  ],
  "links": [
    { "from": "a1", "to": "a2" }, { "from": "a2", "to": "a3" }, { "from": "a3", "to": "a4" },
    { "from": "a4", "to": "a5" }, { "from": "a5", "to": "a6" }, { "from": "a6", "to": "sa" },
    { "from": "sa", "to": "b1" }, { "from": "b1", "to": "b2" }, { "from": "b2", "to": "b3" },
    { "from": "b3", "to": "b4" }, { "from": "b4", "to": "b5" }, { "from": "b5", "to": "b6" },
    { "from": "b6", "to": "sb" }, { "from": "sb", "to": "a1" }
  ]
}
```

The players have no `direction` and no starting pieces (the stones are the places' `start`), and the
one piece, `STONE`, has no moves:

```json
"players": [
  { "name": "SOUTH", "starting_positions": [] },
  { "name": "NORTH", "starting_positions": [] }
],
"turns": { "order": ["SOUTH", "NORTH"], "stages": [ { "name": "AGAIN" } ] },
"pieces": [ { "code": "STONE", "name": "stone", "moves": [] } ]
```

The stage `AGAIN` has no `only`: any way may be played in it (see [Stages](#stages)).

Four zones, each with a part for each player: `PITS` (the mover's six pits), `OTHER_STORE` (the
store of the other player, which a sowing skips), `STORE` (the mover's own store) and `MINE` (a
player's pits and store, which the score counts):

```json
"zones": [
  { "name": "PITS", "places": {
      "SOUTH": ["a1", "a2", "a3", "a4", "a5", "a6"],
      "NORTH": ["b1", "b2", "b3", "b4", "b5", "b6"] } },
  { "name": "OTHER_STORE", "places": { "SOUTH": ["sb"], "NORTH": ["sa"] } },
  { "name": "STORE", "places": { "SOUTH": ["sa"], "NORTH": ["sb"] } },
  { "name": "MINE", "places": {
      "SOUTH": ["a1", "a2", "a3", "a4", "a5", "a6", "sa"],
      "NORTH": ["b1", "b2", "b3", "b4", "b5", "b6", "sb"] } }
]
```

The one action, `SOW`:

```json
"actions": [
  { "name": "SOW", "do": [
    { "choose": "PLACE", "in": { "region": "BOARD" } },
    { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "PITS" },
    { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } } },
    { "effect": "FOR_EACH",
      "in": { "region": "FOLLOW", "from": { "place": "TARGET" },
              "steps": { "count": { "in": { "region": "AT", "at": { "place": "TARGET" } }, "where": { "codes": ["STONE"] } } },
              "skipping": "OTHER_STORE" },
      "do": [
        { "effect": "MOVE", "from": { "place": "TARGET" }, "to": { "place": "EACH" },
          "what": { "codes": ["STONE"] }, "count": 1 }
      ] },
    { "if": { "test": "IN_ZONE", "at": { "place": "LAST" }, "zone": "STORE" },
      "then": [ { "effect": "STAGE", "name": "AGAIN" } ] }
  ] }
]
```

1. The player picks a place of the board, and the first test keeps it to their own pits (the mover's
   part of `PITS`). The second says the pit holds a stone (a `COUNT` with no `min` wants one): an
   empty pit is not offered.
2. The `FOR_EACH` runs once for each place of the `FOLLOW`. The region is taken before the first
   run, so `steps`, the number of stones in the pit, is counted once, with all of them still there.
   The walk goes from the pit along `next`, skipping the other player's store: for SOUTH, `sb` is
   passed over and does not count as a step.
3. Each run moves one stone from the pit (`TARGET`) to the place being visited (`EACH`). With 13 or
   more stones the walk comes back round to the pit itself, and drops one there.
4. `LAST` is where the latest `MOVE` of the turn put a stone: the last place the sowing reached.
   When that is the mover's own store, the `STAGE` effect keeps the turn with them, in `AGAIN`. A
   player who has no stone to sow then has no way, so the turn is handed on.

The game ends when the player to move has no way to play, which is when all their pits are empty,
and the score counts the stones each player keeps, store and pits:

```json
"end_checks": [
  { "as": "NEXT", "when": { "test": "STUCK" }, "then": { "outcome": "HIGHEST" } }
],
"score": [
  { "term": "KEPT", "zone": "MINE", "where": { "codes": ["STONE"] } }
]
```

What this game leaves out is in [Known limitations](#known-limitations): capturing the opposite
pit.

---

## Placing a piece

Some games hand each player as many pieces as they want: stones in Go, discs in Reversi. An action
makes the piece with a `PLACE` effect, so there is nothing to keep in a hand:

```json
{ "name": "STONE", "do": [
  { "choose": "PLACE", "in": { "region": "BOARD" } },
  { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } } } },
  { "effect": "PLACE", "at": { "place": "TARGET" }, "code": "STONE" }
] }
```

The player picks a place of the board (`PLACE` choice in the `BOARD` region), the test keeps the
stone off an occupied point, and the effect creates the stone. `PLACE` replaces whatever stands
on the point, so the test is what makes the point have to be empty. Any test can be added: a
placing is allowed where its tests allow it.

A piece that is only ever placed does not need `moves`; give it an empty list. Placing is a turn
like any other: it advances the turn, is recorded in the history under the action's name, and the
`every_action` entries that apply to it run after it, as after any other action. The bags are a separate
matter: a game can place pieces without a bag (Go), have a hand and no placing (chess variants), or
both.

With two placing actions, say `STONE` and `DISC`, the player says which one by naming the action
(a button) before or after clicking the point.

---

## Passing

A pass is an action with nothing to do:

```json
{ "name": "PASS", "do": [] }
```

A pass is a way of taking an action that changes nothing, and it is played the moment it is
named, as the player's answer. It is recorded in the history under the action's name, with empty
`from` and `to`. Because a player who may pass always has a move, such a game is never a
stalemate. Add `"players": ["BLACK"]` to the action to let only some players pass.

A pass is accepted at the start of a turn, when nothing has been answered. A piece that was
picked up has to be put back first (cancel it); a promotion choice or a jump in progress has to be
finished.

Passing does not end a game by itself. Add an end check on [`PASSES`](#tests) for that, as Go does
(see [Go](#complete-annotated-example--go)).

---

## Liberty capture

Go's capture is written as steps of an [`every_action`](#every_action) entry for the action that
places the stone, so that any game can use it, or change it. `go.json` has:

```json
"every_action": [
  { "for": ["STONE"], "do": [
    { "effect": "FOR_EACH",
      "in": { "region": "AROUND", "of": { "place": "TARGET" }, "through": "orthogonal" },
      "do": [
        { "if": { "test": "ALL", "of": [
            { "test": "COUNT", "in": { "region": "AT", "at": { "place": "EACH" } },
              "where": { "owner": "ENEMY", "codes": ["STONE"] } },
            { "test": "COUNT",
              "in": { "region": "AROUND", "through": "orthogonal",
                      "of": { "region": "GROUP", "of": { "place": "EACH" }, "through": "orthogonal", "where": { "codes": ["STONE"] } } },
              "where": { "empty": true }, "min": 0, "max": 0 } ] },
          "then": [
            { "effect": "FOR_EACH",
              "in": { "region": "GROUP", "of": { "place": "EACH" }, "through": "orthogonal", "where": { "codes": ["STONE"] } },
              "do": [ { "effect": "REMOVE", "at": { "place": "EACH" } } ] } ] }
      ] },
    { "test": "COUNT",
      "in": { "region": "AROUND", "through": "orthogonal",
              "of": { "region": "GROUP", "of": { "place": "TARGET" }, "through": "orthogonal", "where": { "codes": ["STONE"] } } },
      "where": { "empty": true } },
    { "test": "NOT", "of": { "test": "REPEATS" } }
  ] }
]
```

Stones of one player that touch along a row or column form a **group** (`GROUP`; diagonals do
not connect, because `through` is `orthogonal`). The empty points beside a group are its
**liberties**: `AROUND` of a region is the places next to it and not in it, and a `COUNT` with
the `empty` filter counts the empty ones. After the stone is placed:

1. **Captures.** For each orthogonal neighbour of `TARGET` that holds an enemy stone whose group
   has no liberty (`"min": 0, "max": 0`), every stone of that group is removed. The inner
   `FOR_EACH` takes its region before it runs, and inside it `EACH` is the stone being removed.
2. **No suicide.** A test that the group of `TARGET` has a liberty, counting the points the
   captures just freed. So a stone that fills the last liberty of its own group is still allowed
   if it first captures something, which is how a Go capture works. A game that allows suicide
   writes an `if` here instead, which removes the mover's group when it has no liberty:

   ```json
   { "if": { "test": "NOT", "of": { "test": "COUNT",
         "in": { "region": "AROUND", "through": "orthogonal",
                 "of": { "region": "GROUP", "of": { "place": "TARGET" }, "through": "orthogonal", "where": { "codes": ["STONE"] } } },
         "where": { "empty": true } } },
     "then": [
       { "effect": "FOR_EACH",
         "in": { "region": "GROUP", "of": { "place": "TARGET" }, "through": "orthogonal", "where": { "codes": ["STONE"] } },
         "do": [ { "effect": "REMOVE", "at": { "place": "EACH" } } ] } ] }
   ```

3. **Ko.** `NOT` `REPEATS` (see [Repeated positions](#repeated-positions)).

Pieces of a code the `where`s do not name are never captured and are not liberties: they block
the way, like the edge of the board. `PASS` is not named in `for`, so a pass is never refused. A
removed stone counts as captured in the match view like any other captured piece (it does not
enter a bag), and the history marks the move as a capture.

---

## Repeated positions

The test `{ "test": "NOT", "of": { "test": "REPEATS" } }`, as a step of an `every_action` entry,
makes a move illegal when it would leave the position as it was after any earlier turn. The
position is the board, the pieces' state flags and whose turn it is. (A piece's count of moves is
part of it only in a game where some test, filter or flag reads `moved`.) This is the *ko* rule of Go
(as *situational superko*), which stops a capture and its recapture from cycling forever.

It differs from an end check on `REPEATS times n`, which lets the repetition happen and then
declares a draw. A move that is refused is simply left out of the legal moves, so a player can
be left with only `PASS`.

The starting position is not part of the history, so a move that would recreate it is not
refused. In a game where pieces are only added or captured (Go) that cannot happen. A game with
`turns.stages` cannot use `REPEATS`: it is refused when the spec is read.

---

## Complete annotated example — Go

`crates/engine/specs/go.json` is Go on a 9×9 board. Nothing in the engine knows about Go: each
rule is one of the blocks above.

```json
{
  "name": "GO 9X9",
  "board": { "dimensions": [9, 9] },
  "players": [
    { "name": "BLACK", "direction": [[1, 0], [0, 1]], "starting_positions": [] },
    { "name": "WHITE", "direction": [[1, 0], [0, 1]], "starting_positions": [] }
  ],
  "turns": { "order": ["BLACK", "WHITE"] },
  "actions": [
    { "name": "STONE", "do": [
      { "choose": "PLACE", "in": { "region": "BOARD" } },
      { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } } } },
      { "effect": "PLACE", "at": { "place": "TARGET" }, "code": "STONE" }
    ] },
    { "name": "PASS", "do": [] }
  ],
  "every_action": [ { "for": ["STONE"], "do": [ ...as in Liberty capture above... ] } ],
  "pieces": [{ "code": "STONE", "name": "stone", "moves": [] }],
  "score": [
    { "term": "PIECES", "where": { "codes": ["STONE"] } },
    { "term": "TERRITORY", "where": { "codes": ["STONE"] } },
    { "term": "POINTS", "value": 7.5, "to": ["WHITE"] }
  ],
  "end_checks": [
    { "when": { "test": "PASSES", "in_a_row": 2 }, "then": { "outcome": "HIGHEST" } }
  ]
}
```

- The board starts empty (`starting_positions` is empty for both players) and the stone has no
  moves: a turn is a placement or a pass.
- The `STONE` action lets either player place a stone on any empty point, and `PASS` gives up the
  turn.
- The `every_action` entry for `STONE` takes shut-in groups, refuses suicide, and is ko.
- Two passes in a row end the game (`PASSES`), and `HIGHEST` gives it to the better `score`. Each
  player scores their stones plus the empty regions that only their stones touch, and WHITE gets
  7.5 points for moving second, so the game cannot be drawn. For a 13×13 or a 19×19 board, change `dimensions`: `go_13.json` and `go_19.json` do.

---

## Known limitations

- **A `CODE` choice always asks.** There is no "skip" path. To make a promotion *optional*,
  include the piece's own code in the `options` list — choosing it replaces the piece with a fresh
  copy of itself. A promotion with one option is not a choice: write `SET_CODE` with that code.
- **A piece in a bag is chosen, not placed.** `HELD` picks a code from a bag, and `START` then
  names that piece. An offset from `START` cannot be formed, and a `HELD` filter tells pieces apart
  by `codes` only.
- **Bags and holders keep counts, not a pile.** There is no order to what a bag or a place that
  holds many keeps, and no hidden contents. A grid square always holds one piece, and a holder
  place never holds a piece of its own.
- **Kalah has no capture of the opposite pit.** It needs a link label of its own for "the pit
  across", and maps have only the labels listed under [Map boards](#map-boards).
- **Tests don't apply to threats.** A move's tests (but not an empty-`TARGET` test at the start of
  its steps) and its `repeat.until` decide which moves are offered, not what the piece attacks. A piece kept in a zone by `IN_ZONE` tests can
  still project threats outside it. This is intentional for the flying-general rule in Xiangqi
  but should be kept in mind for other designs.
- **Offsets, `PATH` and `LINE` are for grids only.** On a map, tests reach other places through
  `AROUND` and the map's labels.
- **A stage is entered by an effect, and a game with stages cannot use `REPEATS`.** A stage lasts
  one way and is entered again by the next, so "while a test holds" is a stage that enters itself.
  Stages do not look ahead: a check rule over the opponent's next two moves needs more.
- **The Flying Knight is not built.** It is a double move, which is writable already as a turn
  order with a name repeated (`["WHITE", "BLACK", "BLACK"]`), but its check rule ("in check when it
  can be taken within the opponent's two moves"; "the double mover may step into check on the first
  move as long as the second gets out") needs a look two moves ahead, which a stage does not give.
- **`TERRITORY` is a block only Go uses.** Counting the empty regions each player alone surrounds
  needs a way to repeat a rule for each region, which no other game asks for yet.
- **`score` has no dead-piece agreement.** It counts the board as it stands when the players
  pass, so a piece that is dead must be captured first. Scoring that removes dead pieces by
  agreement, or counts prisoners (Japanese rules), is not expressible.
- **A dropped pawn may double-step from any rank.** A dropped piece has never moved, and the
  pawn's double step asks only that, and that the square ahead is empty. Standard Crazyhouse
  allows the double step only from the pawn's second rank.

---

## Old forms

Earlier versions of the format wrote rules as named conditions, and turns as special forms on a
move. Each old form is refused when the spec is read, with an `OldForm` error that names the form
and what replaced it:

| Old form | Write instead |
|----------|---------------|
| The top-level `conditions` list of named zones | `zones`: `{ "name": "PALACE", "places": { "RED": [...], "BLACK": [...] } }` |
| A condition object, with a `condition` field naming it | a [test](#tests), `{ "test": "...", ... }` |
| A piece's `restrictions` | an `IN_ZONE` test at `TARGET` on each of its moves |
| A piece's drop restrictions, which blocked a drop when one held | tests in the `DROP` action, which allow a drop when they all hold |
| A repeat's `until` written as a name, such as `"NEVER"` | a test; leave it out to stop at the first piece, or write `{ "test": "ANY", "of": [] }` never to stop early |
| A side effect's `condition` | a test step before the effect, or an `if` |
| A move's `actions`, keyed by what stands on `TARGET` (`EMPTY` → `MOVE`, `ENEMY` → `CAPTURE`) | the move's `do`: test steps on `TARGET`, then a `MOVE` effect. A move that did different things on an empty and an enemy square becomes two moves with the same `step` |
| A move's `conditions` | test steps at the start of its `do` |
| A move's `modifiers` (a `TRANSFORM` promotion) | an `if` over the zone, with a `CODE` choice and a `SET_CODE` effect in its `then` (or just `SET_CODE`, for one option) |
| A move's `side_effects` (`SET_STATE`, `CAPTURE`, `MOVE`, `CONVERT`, `COPY_SOURCE`, `EXPLODE`) | effect steps: `SET_FLAG`, `REMOVE`, `MOVE`, `SET_OWNER`, `PLACE` and `FOR_EACH` (see [Effects](#effects)) |
| The top-level `allow_pass` | an action with an empty `do`: `{ "name": "PASS", "do": [] }` |
| A piece's `unlimited_supply` | an action that makes the piece with a `PLACE` effect (see [Placing a piece](#placing-a-piece)) |
| The top-level `hand_enabled`, and the capture-to-hand it switched on | `"bags": [ { "name": "HAND", "for": "EACH_PLAYER" } ]`, and `"removed_to": { "bag": "HAND", "of": "MOVER" }` on each effect that removes a piece the mover keeps (see [Bags and the hand](#bags-and-the-hand)) |
| The top-level `hand`, `{ "enabled": true }` | the same: `bags` |
| The top-level `jump_enabled`, and the `END_TURN` effect | a stage in `turns.stages`, and a `STAGE` effect at the end of the way that goes on (in draughts, the `else` of the crowning `if`); see [Stages](#stages) |
| A `HAND` choice, answered `{ "hand": CODE }`, and `"removed_to": { "hand": "MOVER" }` | `{ "choose": "HELD", "in": { "bag": "HAND", "of": "MOVER" } }`, answered `{ "held": CODE }` (`hand` is still read), and `"removed_to": { "bag": "HAND", "of": "MOVER" }`. Refused with a `JsonError`, not an `OldForm` |
| A piece's `demotes_to` | `captured_as`, with the same code |
| A piece's `drop_conditions` | tests in the `DROP` action, after its `HELD` and `PLACE` choices, with `TARGET` the square and `START` the piece in the bag |
| `no_opponent_check` | a closing test in the check entry of [`every_action`](#every_action): `NOT(ATTACKED in BOARD where ENEMY KING by MOVER)` |
| `mandatory_capture` | `"must": [ { "test": "REMOVED", "where": { "owner": "ENEMY" } } ]` (see [`must`](#must)) |
| `liberty_capture`, with its `allow_suicide` | steps of an `every_action` entry for the placing action (see [Liberty capture](#liberty-capture)) |
| `forbid_repeated_positions` | `NOT(REPEATS)` in an `every_action` entry (see [Repeated positions](#repeated-positions)) |
| `win_conditions` (`PIECE_IN_ZONE`, `OPPONENT_BARE`, `CHECK_COUNT`, `AREA_SCORE`, `CONNECTION`) | an [end check](#end_checks) with a test and an outcome: see [What each old rule is written as](#what-each-old-rule-is-written-as) |
| `draw_conditions` (`repetition_count`, `fifty_move_halfmoves`, `fifty_move_pawn_codes`, `insufficient_material`) | end checks that end in `DRAW`: `REPEATS`, a `QUIET` [counter](#counters), a count of the pieces left |
| `leader`, and the checkmate and stalemate it decided | two `STUCK` end checks `as` `NEXT`, `LOSE` where the king is attacked and `DRAW` after it (see [`end_checks`](#end_checks)) |
| `stalemate_loses` | one end check, `as` `NEXT`, `STUCK`, `LOSE` |

The four top-level keys `win_conditions`, `draw_conditions`, `leader` and `stalemate_loses` are
refused in the same way, with an error that names `end_checks`. A spec that had `leader` to keep a
king safe needs the check entry of [`every_action`](#every_action) too: `leader` no longer filtered a
move when it was removed.

The rules the old conditions wrote, as blocks. Here `COUNT in AT(START+[0,1])` is short for
`{ "test": "COUNT", "in": { "region": "AT", "at": { "place": "START", "offset": [0, 1] } } }`:

| Rule | As blocks |
|------|-----------|
| a leg square is empty (the Xiangqi horse) | `NOT(COUNT in AT(START+[0,1]))` |
| the square in front is empty (pawn double step) | `NOT(COUNT in AT(START+[0,1]))` |
| the target holds no ally (a move or a capture) | `NOT(COUNT in AT(TARGET) where owner ALLY)` |
| the target is empty (a move that cannot capture) | `NOT(COUNT in AT(TARGET))` |
| the target holds an enemy (a move that only captures) | `COUNT in AT(TARGET) where owner ENEMY` |
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
| the mover's king is not left attacked (check, in `every_action`) | `NOT(ATTACKED in BOARD where owner ALLY, codes [KING])` |
| the opponent's king is not left attacked (Racing Kings) | `NOT(ATTACKED in BOARD where owner ENEMY, codes [KING] by MOVER)` |
| the way captured (in `must`) | `REMOVED where owner ENEMY` |
| the position after the way is not a repeat (ko) | `NOT(REPEATS)` |
| the move lands in a zone | `IN_ZONE(TARGET, REACH_END)` |
| the player to move has no way to play (in an end check, `as NEXT`) | `STUCK` |
| a piece of the mover's has reached a zone (an end check) | `SOME in BOARD where owner ALLY, codes [KING] that IN_ZONE(EACH, CENTER)` |
| the player has at most one piece left | `COUNT in BOARD where owner ALLY max 1` |
| the way moved a pawn, or took a piece | `ANY[MOVED where codes [PAWN], REMOVED where owner ENEMY]` |
| nothing happened in the last two turns | `PASSES in_a_row 2` |
| a counter has reached a number | `COUNTER CHECKS min 3` |
| no drop in a zone | `NOT(IN_ZONE(TARGET, TERMINAL_RANK))` in the `DROP` action |
| a piece stays in a zone | `IN_ZONE(TARGET, PALACE)` on each of its moves |
| a hand (Shogi, Crazyhouse) | `"bags": [ { "name": "HAND", "for": "EACH_PLAYER" } ]`; `"removed_to": { "bag": "HAND", "of": "MOVER" }` on the `MOVE` or `REMOVE` that takes the piece; a drop is `HELD in HAND of MOVER`, `PLACE in BOARD`, tests, `MOVE START → TARGET` |
| a shared supply or bank | `"bags": [ { "name": "SUPPLY", "start": { "WOOD": 19 } } ]`; `MOVE from SUPPLY to HAND of MOVER what [WOOD] count 2` |
| pieces in a pit (Kalah) | a map place with `capacity` above 1 and a `start`: `{ "name": "a1", "at": [1, 0], "capacity": 48, "start": { "STONE": 4 } }` |
| sowing: one piece to each of the next *n* places | `FOR_EACH in FOLLOW(from TARGET, steps COUNT in AT(TARGET), skipping ZONE) do MOVE from TARGET to EACH what [STONE] count 1` |
| a pit that is not empty | `COUNT in AT(TARGET)` (a holder place counts the pieces it keeps) |
| the pieces a player keeps in their pits and store (a score) | `KEPT zone MINE where codes [STONE]` |

**Design principle**: never add a test that can be expressed by composing existing ones.
