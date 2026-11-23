# 13. Bags and places that hold many pieces

So far a place held one piece, and the only store off the board was the hand of chapter 7. Many
games need more. A Catan player holds a pile of resource cards, and the bank holds the rest. A
clearing in Root holds a dozen warriors. A pit in **Mancala** holds four stones, and the game is
about moving them.

The engine has one idea for all of these, and no rule for any one game: a place that **keeps
counts** of pieces. There are two kinds:

- A **bag** is off the board. The hand is a bag.
- A **map place** with a `capacity` is on the board, and may hold many pieces.

The effects, tests and regions you know speak of both. This chapter shows how, and then builds a
whole game from them: Kalah, the common form of Mancala.

## Bags

A bag is declared at the top level of the spec, as the hand was in chapter 7:

```json
"bags": [
  { "name": "HAND", "for": "EACH_PLAYER" },
  { "name": "SUPPLY", "capacity": 19, "start": { "WOOD": 19 } }
]
```

| Field | Meaning |
|-------|---------|
| `name` | What the effects write. It must be unique, and may not hold an `@`. A bag and a place cannot share a name |
| `for` | `EACH_PLAYER`: every player has a bag of this name. Left out: **one shared bag**, such as a bank |
| `capacity` | The most pieces the bag holds, in all. Left out: no limit |
| `start` | What the bag holds when the game begins, as `{ code: count }`. With `EACH_PLAYER`, every player's bag starts with it |

A bag holds **counts**, such as nine WOOD and two BRICK, in no order. It does not know which wood
is which, so a piece that goes into a bag forgets what happened to it: its moves and its flags.
When it comes out, it is a new, unmoved piece.

A bag is written where a place is written, with `bag` in place of `place`:

```json
{ "bag": "HAND", "of": "MOVER" }
{ "bag": "SUPPLY" }
```

`of` is `MOVER`, `ENEMY` or a player's name, and says whose bag. A shared bag has no `of`, and a
per-player bag needs one. A bag written the wrong way, or one the game does not declare, is
refused when the spec is read.

**Who owns what is kept.** A piece in a player's bag is that player's. A piece in a shared bag, or
in a shared place, belongs to nobody. When it leaves for a place on the board, it becomes the
mover's.

## Places that hold many

On a grid, a square holds one piece, and always will. A game that needs a place with many pieces
writes a **map** (chapter 12), and gives the place a `capacity`:

```json
{ "name": "pit1", "at": [0, 0], "capacity": 48, "start": { "STONE": 4 } }
```

- `capacity` is how many pieces it holds, in all. It is `1` when left out, and that is the place
  you know. Above `1`, the place keeps counts, as a bag does.
- `start` is what it holds when the game begins. It needs a `capacity` above `1`, and may not be
  more than it.
- What such a place holds is a count of pieces of each code, not pieces that move. Give the
  piece an empty `moves` list, as chapter 10 did for a stone.
- The players' `starting_positions` stay `[]`: the pieces are already there, as the `start` of
  the places.

## Moving things: MOVE, REMOVE and PLACE

No new effect was needed. `MOVE`, `REMOVE` and `PLACE` can name a bag, or a place that holds
many, wherever they name a place. For those, they take more fields:

| Field | Meaning |
|-------|---------|
| `what` | Which pieces: a filter with `codes`, `except` and `owner`. Left out: any |
| `count` | How many: a number above `0`, `ALL`, or a counted value (below). Left out: `1` |
| `removed_to` | A bag for the pieces a `REMOVE`, or a `MOVE` that lands on a piece, takes off (chapter 7) |

A `MOVE` does this, by where it goes from and to:

| From | To | What happens |
|------|----|--------------|
| a place | a place | As before |
| a place | a bag, or a place that holds many | The piece becomes a count. It forgets its flags and moves, as in a hand |
| a bag, or a place that holds many | a place | `count` must be `1`. A new, unmoved piece of that code stands there, and is the mover's |
| a bag, or a place that holds many | a bag, or a place that holds many | `count` of the pieces that match move across |

`PLACE` makes new pieces: with a `count`, it puts that many in a bag. `REMOVE` takes `count` of
them out. Three small actions show it, using the bags above:

```json
{ "name": "GATHER", "do": [
  { "effect": "MOVE", "from": { "bag": "SUPPLY" }, "to": { "bag": "HAND", "of": "MOVER" },
    "what": { "codes": ["WOOD"] }, "count": 2 }
] },
{ "name": "BUILD", "do": [
  { "choose": "PLACE", "in": { "region": "BOARD", "where": { "empty": true } } },
  { "effect": "MOVE", "from": { "bag": "HAND", "of": "MOVER" }, "to": { "place": "TARGET" },
    "what": { "codes": ["WOOD"] } }
] },
{ "name": "BURN", "do": [
  { "effect": "REMOVE", "at": { "bag": "SUPPLY" }, "count": 4,
    "removed_to": { "bag": "HAND", "of": "MOVER" } }
] }
```

- **`GATHER`** takes two WOOD from the shared supply into the mover's hand. It has no choice to
  make, so it is just the one effect.
- **`BUILD`** asks for an empty place of the board, and stands one WOOD from the mover's hand on
  it. It is the drop of chapter 7, written with a `MOVE` from a bag, and the placing of chapter
  10, with the piece coming from a store.
- **`BURN`** takes four WOOD out of the supply and sends them to the mover's hand, as
  `removed_to` does for a capture.

### When it does not fit

An effect that would pass a `capacity`, or that takes more than a bag holds, **fails**, and the
way is not offered, as when a test fails. So `GATHER` stops being offered when the supply has
fewer than two WOOD, and a `PLACE` into a full supply is not offered either. No test is needed
for it. (This also holds for a capture whose `removed_to` bag is full: the capture is not offered.)

### How many: the `count` forms

`count` here, and `steps` below, are **counts**. A count is written in one of three ways:

| Form | Meaning |
|------|---------|
| `2` | That many. A number above `0` |
| `"ALL"` | Everything that matches |
| `{ "count": { "in": REGION, "where": FILTER } }` | **What the position says**: the `COUNT` test's own counting, as a value |

The third reads the position when the effect runs. Inside it, `in` is a region and `where` a
filter, as in the `COUNT` test of chapter 4, and the number found is the count. This one asks for
as many stones as the place at `TARGET` holds:

```json
{ "count": { "in": { "region": "AT", "at": { "place": "TARGET" } },
             "where": { "codes": ["STONE"] } } }
```

## Following links: `FOLLOW`

Mancala sows stones one by one along a ring. For that, a region has to be a *walk*: the next
place, then the one after it. A new region does it. It works on maps only:

```json
{ "region": "FOLLOW", "from": { "place": "TARGET" }, "steps": 5,
  "through": "next", "skipping": "OTHER_STORE" }
```

It is the next `steps` places, found by following the label `through` (`next` when left out)
from `from`. When the links go round in a ring, so does the walk, and a long one goes round more
than once. `steps` is a count (above), so it can be read from the position.

`skipping` is optional. It names a zone (chapter 6), and the walk passes over the places of that
zone **for the mover** without counting them. This is how a player's sowing skips the other
player's store.

Each place on the walk needs one place after it, so a link that leads on to several places is
refused when the spec is read, as a `steps` of `0` or `ALL` is. A `FOR_EACH` over a `FOLLOW`
visits the places in order.

### One-way links

The links of chapter 12 were two-way: `["a", "b"]` lets a piece go from `a` to `b` and from `b`
to `a`. A ring that stones go round one way needs links that go one way. They are written as an
object:

```json
"links": [
  { "from": "a1", "to": "a2" },
  ["x", "y"]
]
```

`{ "from": ..., "to": ... }` goes only from `from` to `to`, and gives a `next` link in that
direction. A two-way link keeps its short form, and a map may use both.

## The score term `KEPT`

A game that counts what its places hold needs a score for it (chapter 5 gave the `score` list and
`HIGHEST`). The term `KEPT` counts the pieces in the places of a zone, what they hold included:

```json
{ "term": "KEPT", "zone": "MINE", "where": { "codes": ["STONE"] } }
```

For each player, it counts the pieces that match `where` in the places the zone lists for that
player. A zone has places for each player (chapter 3), so one term scores both sides.

## Kalah

**The rule.** Each player has six pits and a store. Four stones lie in every pit. On your turn you
take all the stones from one of your pits and **sow** them, one in each place that follows, going
round: your pits, your store, the other player's pits, and on. You skip the other player's
store. When a player has no stones in their pits, the game ends, and each player's score is the
stones in their pits and their store. If your last stone lands in your own store, you move again.

Mancala has more rules than that, and Kalah is only one of its games. **The spec in the
repository is not Kalah as it is played.** See "What this Kalah lacks" below. It is a test game,
`crates/engine/src/tests/kalah/spec.json`, written to show that the blocks above are enough for a
game where the pieces are not on squares at all.

### The board

Twelve pits and two stores, in a ring of fourteen places. South's pits `a1` to `a6` run along the
bottom, South's store `sa` is at the right, North's `b1` to `b6` come back along the top, and
North's store `sb` is at the left.

```json
"board": {
  "shape": "map",
  "places": [
    { "name": "a1", "at": [1, 0], "capacity": 48, "start": { "STONE": 4 } },
    { "name": "a2", "at": [2, 0], "capacity": 48, "start": { "STONE": 4 } },
    ...
    { "name": "sa", "at": [7, 1], "capacity": 48 },
    { "name": "b1", "at": [6, 2], "capacity": 48, "start": { "STONE": 4 } },
    ...
    { "name": "sb", "at": [0, 1], "capacity": 48 }
  ],
  "links": [
    { "from": "a1", "to": "a2" },
    { "from": "a2", "to": "a3" },
    ...
    { "from": "a6", "to": "sa" },
    { "from": "sa", "to": "b1" },
    ...
    { "from": "b6", "to": "sb" },
    { "from": "sb", "to": "a1" }
  ]
}
```

(The `...` stand for places and links written the same way. The real file has all fourteen.)
Each pit starts with four stones, and `capacity` is 48, the number of stones in the game: no place
can hold more than all of them. The links all go one way, so `next` from `a6` is the store `sa`,
and from `sb` it is `a1`: a ring.

The players have no pieces and no `direction`, and the one piece is a stone that never moves on
its own:

```json
"players": [
  { "name": "SOUTH", "starting_positions": [] },
  { "name": "NORTH", "starting_positions": [] }
],
"turns": { "order": ["SOUTH", "NORTH"], "stages": [ { "name": "AGAIN" } ] },
"pieces": [ { "code": "STONE", "name": "stone", "moves": [] } ]
```

The stones start in places that are shared, so they belong to nobody. Nothing in this game needs
them to. The stage `AGAIN` in `turns` is for the free turn, further down.

### The zones

Four zones say what belongs to whom:

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

`PITS` are a player's own pits. `OTHER_STORE` is the store that player may not sow into (for
SOUTH, North's store `sb`). `STORE` is the store a player sows into and keeps. `MINE` is a player's
pits and store together.

### The turn: SOW

A turn is one action, made of blocks you have met:

```json
{ "name": "SOW", "do": [
  { "choose": "PLACE", "in": { "region": "BOARD" } },
  { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "PITS" },
  { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } } },
  { "effect": "FOR_EACH",
    "in": { "region": "FOLLOW", "from": { "place": "TARGET" },
            "steps": { "count": { "in": { "region": "AT", "at": { "place": "TARGET" } },
                                  "where": { "codes": ["STONE"] } } },
            "skipping": "OTHER_STORE" },
    "do": [
      { "effect": "MOVE", "from": { "place": "TARGET" }, "to": { "place": "EACH" },
        "what": { "codes": ["STONE"] }, "count": 1 }
    ] },
  { "if": { "test": "IN_ZONE", "at": { "place": "LAST" }, "zone": "STORE" },
    "then": [ { "effect": "STAGE", "name": "AGAIN" } ] }
] }
```

It reads in order:

- **`choose PLACE`** asks for a place. Every place of the board is a candidate, and the tests
  narrow them.
- **`IN_ZONE ... PITS`** keeps the mover's own pits. A zone is seen from the mover, so South is
  offered `a1` to `a6` and North `b1` to `b6`.
- **`COUNT`** asks for at least one piece at `TARGET`, so an empty pit is not offered.
- **`FOR_EACH` over a `FOLLOW`** is the sowing. The walk starts at the chosen pit and goes
  `steps` places, and `steps` is a counted value: *the number of stones in that pit*. It goes
  round the ring, and passes over the other player's store.
- **The `MOVE`** inside moves one stone from the chosen pit to the place the walk is at, `EACH`.
  After `steps` of them, the pit is empty.
- **The `if`** is the free turn. `LAST` is the place where the latest `MOVE` of the turn put a
  piece, here the place the sowing ended at. When that is the mover's own store (`STORE`), the
  `STAGE` effect keeps the turn with them, in the stage `AGAIN`, which has no `only` and so allows
  every way: they sow again. A player with no stone left to sow has no way, so the turn is handed
  on; the end check below is judged for the player it is handed to, and ends the game when that
  player has no stone either.

The walk is worked out before the stones move, so it is as long as the pit was. A pit with
thirteen stones or more goes all the way round, and the pit it left gets a stone as well.

### The end

The game ends when the player to move has nothing to sow. That is the `STUCK` test (chapter 5),
and the score decides:

```json
"end_checks": [
  { "as": "NEXT", "when": { "test": "STUCK" }, "then": { "outcome": "HIGHEST" } }
],
"score": [
  { "term": "KEPT", "zone": "MINE", "where": { "codes": ["STONE"] } }
]
```

`"as": "NEXT"` judges the position as the player about to move sees it, so a player who has no
stones to sow when their turn comes ends the game. `HIGHEST` gives the win to the highest score.
The score is `KEPT` over `MINE`: the stones in a player's six pits and their store. So the stones
left on a side count for that side, as the rules say.

That is the whole game: one action, one stage, one end check, one score. No part of the engine knows what
Kalah is.

### What this Kalah lacks

It is not a catalogue game, because a rule of the real one is missing:

- **Capturing the opposite pit.** If your last stone lands in an empty pit of yours, and the pit
  across the board holds stones, you take them and your last stone into your store. The place
  where the sowing ended is there now (`LAST`), but the pit across is not: it needs a link of a
  kind of its own that joins each pit to the pit across from it, and today a map has `next` and
  the labels of chapter 12, and a spec cannot name a new one. This is not built.

The free turn is built, as a stage. The Flying Knight is still not in the catalogue: it is a
double move, which a turn order with a name repeated already gives, but its check rule needs a
look two moves ahead, which a stage does not give. See [Known limitations](known-limitations.md).

So this game is a little more peaceful than Kalah. The bag, the place that
holds many, the counted `steps` and the ring of one-way links are the real thing.

## Try it

There is no board for this one in the guide, and the command-line player draws grids only. The
tests play the game:

```sh
cargo test -p modern-chess-engine kalah
```

Each test is one idea from the list above: the starting stones, a sowing that passes a store, a
sowing that skips the other store and goes round, a pit that cannot be chosen when empty, a last
stone in your own store that gives another turn, and the end of the game. Read them with the spec beside you. The tests of the bags themselves are in
`crates/engine/src/tests/bags/mod.rs`.

## Now change it

Make a scratch copy of `crates/engine/src/tests/kalah/spec.json`, and change one thing at a time.

1. **More stones.** Put `6` stones in every pit instead of `4`. 72 stones are now in play, and
   `capacity` is 48. A sowing that would pass a place's capacity is not offered, so sooner or later
   a pit has no way to be sown. Raise every `capacity` to 72 and it is a game again.
2. **A five-pit game.** Remove `a6` and `b6` from the places, the links and the zones. Close the
   rings again: `a5` links to `sa`, and `b5` to `sb`. How many stones are there now, and what should
   `capacity` be?
3. **A second kind of seed.** Add a piece `GEM`, with `"moves": []`, and write
   `"start": { "STONE": 4, "GEM": 1 }` on `a1`. Sowing `a1` moves stones only, because the `MOVE`
   says `"codes": ["STONE"]`, and `steps` counts stones only. Where does the GEM stay? Then add a
   second `KEPT` term with `"codes": ["GEM"]` and see the score change.
4. **A different sowing.** Change `"skipping": "OTHER_STORE"` to no `skipping` at all. Now each
   player's sowing drops stones in both stores. Which player does that help?
