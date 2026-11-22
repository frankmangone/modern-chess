# Map boards (step 4b)

## Why

Step 4 of the engine route (`2026-11-21-engine-generalisation-design.md`) brings boards that are
not grids. Frank's benchmarks, 2026-11-21: Catan and Root, "at least as benchmarks for what can be
built". Pieces must be able to sit on points (Catan's corners, Root's clearings), on paths (roads,
the paths between clearings) and on areas (tiles with the robber, forests), so a map is not a
Catan special case. Step 4a moved every layer outside the engine to node names, so a map plugs in
without changing how positions travel.

## Goals

- A spec can write a map: places, each a point, a path or an area, and the links between them.
- A spec can generate the Catan shape from a hex layout: tiles, their corners and their sides.
- Pieces sit on, and move between, places of all three kinds.
- The board element draws a map (dots, lines, shapes) and answers clicks with place names.
- **Grid games stay as fast as they are.** Frank, 2026-11-22: "on simpler games, execution should
  remain fast". Measured, not assumed (see Proving it).
- Every existing game plays and looks exactly as it does today.

## Non-goals

- Many pieces on one place (Root's warriors in a clearing): step 4c, with a place's capacity.
- Building rules (Catan's distance rule, roads that must connect): the rule vocabulary, with the
  generality pass Frank wants to do together.
- Links that mean something different for each player (a map's "forward"). Neither benchmark needs
  them; maps have no player orientation.
- Sliding along a link (a move repeated along `next`). A link can lead to several places, so a
  repeat has no single meaning yet; refused on maps for now.
- Readable labels (`e4`) and drawing coordinates written for grids.

## Anti-goals

- Slowing grid games down to make maps possible.
- A map rule that silently does nothing: a rule that cannot mean anything on a map is refused when
  the spec is read, with a readable error.
- Catan-specific code. The generated hex map is plain places and links like any written map.

## Approaches

- **Maps as a second kind of board (chosen).** Grids keep today's walk: no stored edges, one
  lookup in a dense table per step. A map stores its links as tables, one per label, giving each
  place its linked places. A grid game pays at most one check per step for which kind of board it
  is. Frank, 2026-11-22: "if preserving the board approach is faster and makes for simpler code, we
  definitely go with option 1. I'll let you decide, based on the speed goal." Claude's call: this
  one, since it leaves the grid's hot path alone, and the 5% budget below holds it to that.
- **Every board stored as a map, grids included.** One code path, but every vector a rule uses
  becomes a stored table over every square; step 2 avoided this because specs with thousands of
  moves or many dimensions grow large and slow to build. Ruled out on speed.
- **Shadow boards** (Frank's idea, 2026-11-22): a second grid laid over the first whose cells are
  the first one's corners. Covers Catan, not Root, and Frank called it a hack. Ruled out.

## The design

### Places and links

A map is a list of places. Each has a name and a kind:

| Kind | Benchmarks | Written as |
|---|---|---|
| point | Catan's corners, Root's clearings | `{ "name": "a", "at": [x, y] }` (kind defaults to point) |
| path | Catan's sides (roads), Root's paths | `{ "name": "ab", "kind": "path", "ends": ["a", "b"] }` |
| area | Catan's tiles, Root's forests | `{ "name": "f", "kind": "area", "around": ["a", "b", "c"] }` |

- `at` is the drawing position in the map's own units, any numbers. A path is drawn between its
  ends and an area as the outline its `around` points make, in that order, so only points need
  `at`; a path's `at` is its midpoint and an area's the average of its corners (Claude's call:
  where its piece is drawn).
- `ends` are two different points. `around` is three or more different points.

Links have labels. The kinds give a map these links without writing them (Claude's call on the
names):

| Label | From | To |
|---|---|---|
| `next` | a point | the points joined to it by a path, or by a written link |
| `next` | a path | the paths sharing one of its ends |
| `next` | an area | the areas sharing a side with it (two outline points in a row) |
| `ends` | a path | its two points |
| `paths` | a point | the paths that end at it |
| `corners` | an area | its outline points |
| `areas` | a point or a path | the areas it is on (for a path: whose outline has its ends in a row) |
| `sides` | an area | the paths along its outline |

A spec may add `next` links between points with no path place between them:
`"links": [["a", "b"], ...]`, each both ways (a classic board drawn as dots and lines, like Pong
hau k'i). Claude's call: written links are `next` only and two-way in 4b; other labels and one-way
links wait for a game that needs them.

The neighbourhoods the rules already use, `orthogonal` and `adjacent` (Go's liberties, Hex's
connections, explosions, `ALLY_ADJACENT_COUNT`), are both `next` on a map.

### The spec

```json
"board": {
  "shape": "map",
  "places": [ { "name": "a", "at": [0, 0] }, { "name": "ab", "kind": "path", "ends": ["a", "b"] } ],
  "links": [ ["a", "c"] ]
}
```

A generated hex map: `{ "shape": "hex-map", "layout": "hexagon", "size": 3 }`, with the same
`layout`, `size` and optional `disabled_positions` as a hex board. It compiles, when the spec is
read, into a map:

- each tile is an area named as on a hex board (`"2,3"`);
- each corner of a tile is a point, named by the three tiles that meet there, on the board or
  not, sorted and joined by `|` (`"1,2|2,2|2,3"`);
- each side is a path named by its two tiles (`"2,2|2,3"`).

Claude's call on the names: an author can work them out from the tile coordinates, and they are
unique. Drawing positions follow the board element's hex orientation, so a hex map's tiles sit
where a hex board's cells would.

Starting positions and global `POSITION` conditions accept a place name on any board, as 4a
made names the one form outside the engine, and coordinates on grids only (Claude's call: one
rule for every board instead of two). On a map every name must be a place. A written map takes
no `disabled_positions` (leave the place out instead); a hex map's `disabled_positions` are tile
coordinates, as on a hex board. A move's `step` is either a vector (grids) or a label (maps): `"step": "next"`. A
move along a label goes to each place the label links to, one move per place.

The checker refuses, with a readable error:

- duplicate or empty place names, or names longer than 64 bytes (the game service's limit);
- `ends` or `around` naming an unknown place or a place that is not a point, a path whose ends are
  the same, an area with fewer than three corners; more than 10,000 places; an empty map;
- on a map, anything written as a vector: a vector `step`, a condition's `position`, a side
  effect's `from`, `to` or `target`, a player `direction`, `ALLY_ON_FILE`; a `repeat` on a label
  step;
- on a grid, a label `step`; on a map, coordinates where a place is due;
- a label step naming a label maps do not have.

### The engine

- `Graph` gains a map form beside the grid one. Grids walk as today. A map keeps, per label, each
  place's linked places in one flat table (offsets into a list), so reading a link is one lookup.
- `Labels` interns names as well as vectors. Vector labels are only ever followed on grids and
  name labels only on maps, which the checker guarantees.
- Move generation: a label step yields one candidate per linked place, each judged exactly as a
  one-step move is today. Grid moves take today's path unchanged.
- Board order on a map is the order the spec writes the places (generated maps: tiles, then
  corners, then sides, each sorted by name). Claude's call.

### The view, the wasm wrapper, the session

- `shape` gains `"map"`.
- Each node keeps `name` and `at`; `at` may now be fractional (grids still send whole numbers).
  Map nodes add `kind`, and `ends` (path) or `around` (area).
- A map view adds `lines`: the written `next` links between points, which no path place draws.
- Nothing else changes: positions are already names (4a).

### The board element

A map is drawn in an SVG scaled to its places' bounds: areas as filled shapes under everything,
then `lines`, then paths as thick lines, then points as dots. Pieces sit at a place's `at`, sized
from the closest two points. Every place is clickable, and selection, destinations and zone tints
mark it as on a grid. A flipped map turns 180°. Skin decorations (star points, rivers) are not
drawn on maps (4a's note that decorations assume bounds starting at 0,0 stays for grids).

### Proof

- **Pong hau k'i**, the classic Chinese blocking game: five points, seven links, two pieces each;
  a piece steps along `next` to the empty point; a player who cannot move loses
  (`stalemate_loses`, no leader). Listed in the catalogue, family "Blocking games" (Claude's
  call). A repetition draw stops endless play.
- **A map chapter at the end of the docs book**: Pong hau k'i to teach places and
  links, then a Catan-shaped hex map (`hexagon`, size 3) in "Try it" with a robber on a tile, a
  settlement on a corner and a road on a side, each stepping along `next`. Not a game; it shows
  that every kind of place holds and moves a piece. Claude's call: the proof that paths and areas
  work, since no classic game with today's rules uses them.

## Proving it

- **Speed, before anything else changes:** a timing run in release mode counts every legal move a
  few plies deep from the start of chess, Shogi and 9×9 Go, five runs each, and records the
  medians. After 4b, the same run on the same machine stays within 5% of each median. If it does
  not, 4b is not done.
- Every existing test keeps what it checks.
- New engine tests: a written map's places and links, each implied label; the hex map's counts
  (a size-3 hexagon has 19 tiles, 54 corners, 72 sides) and names; a label step's moves; Go's
  liberties and a connection on a map; each refusal above.
- The view lists a map's places with kinds, ends, outlines and lines.
- The board element's model tests cover map bounds and piece placement on each kind.
- In a browser: Pong hau k'i played to a win in "Try it" and through the game service; the Catan
  sampler's three pieces moved; every existing board style unchanged.

## Left to the builder

- Type names, file layout, how the flat link tables are built.
- How the timing run is written and where it lives, as long as it is repeatable from one command.
- Error texts, in the style of today's.
- Piece and dot sizes, line widths and colours on a map, from the skin where it has them.
