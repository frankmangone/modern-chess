# 11. Maps: places and links

Every board so far was a grid. A piece sat on a square, and a move was an offset such as
`[0, 1]`. Some games have no grid. **Pong hau k'i** is five dots joined by lines. **Catan** has
tiles, the corners where tiles meet, and the sides between them. This chapter shows how to
write a board like that, as a **map**.

A map is a list of named **places**, and the **links** between them. A move no longer says
"one square up". It says "to the next place along a link".

## Places and their kinds

```json
"board": {
  "shape": "map",
  "places": [
    { "name": "a", "at": [0, 0] },
    { "name": "b", "at": [2, 0] },
    { "name": "c", "at": [1, 2] },
    { "name": "ab", "kind": "path", "ends": ["a", "b"] },
    { "name": "f", "kind": "area", "around": ["a", "b", "c"] }
  ],
  "links": [["a", "c"]]
}
```

Every place has a `name`, from 1 to 64 bytes. There are three kinds, and all three can hold a
piece:

- A **point** is a dot. This is the default, and the only kind that needs `at`: where it is
  drawn, in any units you like. Catan's corners are points.
- A **path** runs between two points, its `ends`. Catan's sides are paths, and a road sits on
  one.
- An **area** is the shape around three or more points, its `around`. Catan's tiles are areas,
  and the robber sits on one.

A path is drawn between its ends, and an area as the outline its points make in the order
written. So you only give `at` for points.

## Links and labels

The kinds give you links without writing them. Each link has a **label**:

| Label | From | To |
|-------|------|----|
| `next` | a point | the points joined to it by a path or a written link |
| `next` | a path | the paths that share one of its ends |
| `next` | an area | the areas that share a side with it |
| `ends` | a path | its two points |
| `paths` | a point | the paths that end at it |
| `corners` | an area | the points around it |
| `areas` | a point or a path | the areas it lies on |
| `sides` | an area | the paths along its edge |

`next` is the one most games use. It is "the places beside this one", whatever kind the place
is. The `links` list adds `next` links between points that have no path between them, each
one both ways. A board drawn as dots and lines needs nothing else. (Chapter 12 adds links that go
one way.)

A link may also carry a label of its own, such as `across`, so a rule can follow it. Written as
`{ "from": "a3", "to": "b4", "label": "across" }` it goes one way, and as `{ "between": ["a3",
"b4"], "label": "across" }` both ways. A step, an `AROUND` or a `FOLLOW` may then name `across`.
A label is 1 to 64 bytes, a map writes at most 64 of them, and none is one of the five labels the
engine works out (`ends`, `paths`, `corners`, `areas`, `sides`) or `adjacent` or `orthogonal`. A
link with a label of its own is not drawn as a line. The Kalah of chapter 12 uses two.

## Moving along a link

```json
{
  "id": 0,
  "step": "next",
  "do": [
    { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } } } },
    { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } }
  ]
}
```

A move whose `step` is a label goes to every place the label links to, and each one is a
separate move. The `do` judges each place as it judges a square. Here the stone may move
only to an empty place.

Two more things change on a map:

- A position is a **place name**, not a pair of numbers: `"positions": ["centre"]`. This holds
  for `starting_positions` and for the places of `zones`.
- The neighbourhoods that other rules use, `orthogonal` and `adjacent`, both mean `next`. So
  liberties, connections, explosions and a test's `AROUND` work on a map. An `AROUND` may also go
  `through` any of the labels above.

A player on a map has no `direction`, so leave it out. The engine refuses anything that only
makes sense on a grid, and the error names the problem:

- a `step` that is an offset such as `[0, 1]`, and `repeat`;
- an `offset` on a place, in a test or an effect, and a test's `PATH` or `LINE`, which need a grid's
  straight lines;
- `direction`;
- a label that the map does not have (its own labels are the ones its links write).

The other way round, a label `step` is refused on a grid.

## Pong hau k'i

A classic Chinese game for two. The board has five points: the four corners of a square and its
centre. Lines join each corner to the centre, and the left, right and bottom sides. There is
no line along the top.

```json
"board": {
  "shape": "map",
  "places": [
    { "name": "top-left", "at": [0, 2] },
    { "name": "top-right", "at": [2, 2] },
    { "name": "centre", "at": [1, 1] },
    { "name": "bottom-left", "at": [0, 0] },
    { "name": "bottom-right", "at": [2, 0] }
  ],
  "links": [
    ["top-left", "centre"], ["top-right", "centre"],
    ["bottom-left", "centre"], ["bottom-right", "centre"],
    ["top-left", "bottom-left"], ["top-right", "bottom-right"],
    ["bottom-left", "bottom-right"]
  ]
}
```

The spec is `crates/engine/specs/pong_hau_ki.json`. Each player has two stones. BLACK starts on
the top corners and WHITE on the bottom ones, so the centre is empty, and WHITE moves first.
There is one piece, which steps along `next` onto an empty place, and nothing is ever captured.

The aim comes from two end checks you have met. The one with `STUCK`, judged `as` `NEXT`, makes the
player with no legal move lose. The one with `REPEATS` and `times: 3` makes a position that comes
up three times a draw. Without it the game could go round in circles.

```json
"end_checks": [
  { "when": { "test": "REPEATS", "times": 3 }, "then": { "outcome": "DRAW" } },
  { "as": "NEXT", "when": { "test": "STUCK" }, "then": { "outcome": "LOSE" } }
]
```

The repeat counts the stones and whose turn it is, and nothing else: no rule of this game asks
whether a stone has moved, so how often a stone has moved is no part of the position.

## Try it

Click a stone to see where it can go, then click an empty point. WHITE moves first. Try to leave
BLACK with both stones shut in.

<modern-chess-board spec="boards/pong_hau_ki.json" skin="boards/pong_hau_ki.skin.json">Playable board (needs JavaScript).</modern-chess-board>

You will see that a stone can only slide along a line, and that there is never more than one
empty point, so a player often has only one or two moves.

## A map made from tiles

Writing a Catan board by hand would take 145 places. A **hex map** makes them for you, from the
same `layout` and `size` as a hex board:

```json
"board": { "shape": "hex-map", "layout": "hexagon", "size": 3 }
```

A size-3 hexagon has 19 tiles, 54 corners and 72 sides. Each place gets a name you can work out:

- A tile is an area, named by its coordinates: `"2,2"`.
- A side is a path, named by its two tiles: `"2,2|2,3"`.
- A corner is a point, named by the three tiles that meet there: `"1,1|1,2|2,2"`.

The tiles in a name are sorted by their first number, then the second, and joined with `|`. A
corner at the edge can touch a tile that is off the board, so a name may hold a negative number,
such as `"-1,0|0,0|0,1"`.

The spec `boards/catan_sampler.json` is not a game. It puts one of each kind of piece on this
board: a `ROBBER` on the tile `"2,2"`, a `SETTLEMENT` on one of its corners, and a `ROAD` on one
of its sides. Each steps along `next` onto an empty place:

```json
{ "piece": "ROBBER", "positions": ["2,2"] },
{ "piece": "SETTLEMENT", "positions": ["1,1|1,2|2,2"] },
{ "piece": "ROAD", "positions": ["2,2|2,3"] }
```

Because `next` means something different for each kind of place, the three pieces move in three
different ways.

## Try it: the sampler

Click the robber and it can go to a neighbouring tile. Click the settlement and it can go to a
neighbouring corner. Click the road and it can go to a road that shares an end with it. There is
only one player, so you can keep moving.

<modern-chess-board spec="boards/catan_sampler.json" skin="boards/catan_sampler.skin.json">Playable board (needs JavaScript).</modern-chess-board>

## Now change it

1. In Pong hau k'i, add the link `["top-left", "top-right"]`, so there is a line along the top.
   Play it again. What changes?
2. Remove the link `["bottom-left", "bottom-right"]` instead. Can WHITE still make the same
   first moves?
3. In the sampler, give the robber `"step": "corners"` instead of `"next"`. It now moves from
   its tile onto the corners around it.
4. Give the settlement `"step": "areas"`. It now moves from its corner onto the tiles it
   touches.
