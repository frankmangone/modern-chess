# Places that hold many pieces, and bags (step 4c-2)

Second half of step 4c (the first, `2026-11-23-hand-effects-design.md`, is merged as #77).

## Intent

Today one square holds one piece, and the hand is the only place that holds several, wired in as
a special case. Frank's benchmarks are Catan (resource hands, a bank) and Root (many warriors in
a clearing, a supply), and his rule for this pass: *anything that can be built from general
blocks is built from them*. So 4c-2 adds no rule for any one game. It adds one idea, a **holder**
(a place that keeps counts of pieces), and widens the blocks that already exist so that they can
speak of it.

## Decided with Frank (2026-11-23/24)

- **Words.** A **place** is on the board. A **bag** is a holder off the board. (*Zone* is taken:
  it is already a named set of squares.)
- **The hand is a bag.** It is the bag `HAND`, one per player. `hand` as a key goes away
  (option B): a game that has a hand declares that bag, and `removed_to` always names a bag.
- **Bags hold counts only** (per player and code), not an ordered pile; decks wait for step 7.
- **Counters stay plain numbers.**
- **The proof is Kalah (Mancala)**, played by the blocks, not by a rule for it.

## Goals, non-goals, anti-goals

Goals: a holder place; bags; every effect, test and region that speaks of a place can speak of a
holder; sowing, recruiting and trading are all written from those blocks; Shogi and Crazyhouse
play exactly as now; grid games stay within 5% of `main`.

Non-goals: ordered piles and hidden contents (step 7); turn stages, so Kalah's free turn after a
last stone in the store waits for step 5, like the Flying Knight; a holder on a grid square (a
grid square holds one piece; a game that needs a many-piece place writes a map); ordinary pieces
standing on a holder; a new kind of piece movement.

Anti-goals: a block only Kalah could use; a spec that silently means something else (a holder
used where it cannot work is refused when the spec is read); slowing the hot path.

## The design

**One store.** The engine keeps `holders: IdMap<Position, Holder>`, next to `pieces`. A `Holder`
is a short list of `(owner, code, count)`; the owner may be nobody (a bank's wood, Kalah's
stones). A bag is a holder whose name is not on the board (a per-player bag is named
`NAME@PLAYER`; `@` is refused in names). A map place with `capacity` above 1 is a holder on the
board. A place with no capacity holds one piece, as today, in `pieces`, so grid games pay
nothing: the holder store is empty and every lookup of it is skipped.

**Spec.**

```json
"bags": [ { "name": "HAND", "for": "EACH_PLAYER" },
          { "name": "SUPPLY", "start": { "WOOD": 19 } } ]
"places": [ { "name": "pit1", "at": [0, 0], "capacity": 99, "start": { "STONE": 4 } } ]
```

- A bag is `for` `EACH_PLAYER` or shared (the default); its capacity is unlimited unless written;
  `start` is its starting contents, for each owner of the bag (a shared bag's are owned by
  nobody).
- A map place's `capacity` (default 1; above 1 makes it a holder) and `start` are written on the
  place. Read-time refusals: `start` on a place of capacity 1, a start above the capacity, a name
  used twice by a place and a bag.
- A place is written `{ "bag": "HAND", "of": "MOVER" }` (`of`: `MOVER`, `ENEMY`, a player name;
  absent for a shared bag), anywhere a `{ "place": ... }` is written today.

**Effects (no new kind).** `MOVE`, `PLACE` and `REMOVE` can name a holder place, and take two
optional fields for it: `what` (a filter: `codes`, `owner`) and `count` (a number, `ALL`, or a
count read from the position, below).

| from → to | meaning |
|-----------|---------|
| place → place | as today |
| place → holder | the piece becomes a count (it forgets its flags and moves, as in a hand today) |
| holder → place | `count` must be 1; a new, unmoved piece of the code (nobody's pieces become the mover's) |
| holder → holder | `count` of what matches, owners in turn order, move across |

`removed_to` is the same move for the piece an effect removes: `{ "bag": ..., "of": ... }`, in
place of `{ "hand": "MOVER" }`. An effect that would pass a holder's capacity, or take more than
it holds, fails: the way is not offered, as when a test fails.

**Numbers and regions.** A count is a literal, `ALL`, or `{ "count": { "in": R, "where": F } }`:
the `COUNT` test's own counting, as a value. A new region `FOLLOW { from, through, steps,
skipping }` is the `steps` places met by following a map label from `from`, going round when it
loops (a link that leads to several places is refused when the spec is read), without the places
of the zone named by `skipping` for the mover. `FOR_EACH` sets a place `LAST` to the last place
it visited. `COUNT` and `Board`'s `empty` read holders (a holder is empty at count 0).

**Choice and answer.** `choose: HAND` becomes `{ "choose": "HELD", "in": { "bag": ... },
"where": { "codes": [...] } }`, answered `{ "held": CODE }` (was `{ "hand": CODE }`); `START` is
still the held piece, for the tests of a drop. A saved game's `hand` loads into the bags `HAND@P`.

**View.** The session view lists `bags` (name, owner, contents) in place of `hands`/`hand`, and a
holder place's pieces with a `count`. The web board shows a count on a place's piece and the bags
in the tray the hand has now.

**Kalah, from these blocks.** Fourteen map places in a ring (`next`), pits with capacity 99 and
four neutral stones each, stores as places beside them. A turn: choose a pit of yours that holds
stones; `FOR_EACH` over `FOLLOW` from it, with `steps` the stones in the pit and `skipping` the
other player's store, moves one stone from the pit to each place; then, if `LAST` is an empty pit
of yours and the opposite pit holds stones, both go to your store. The game ends when a player
has no stones in their pits; the score counts the stones in a player's zone. Not a catalogue
game: it lacks the free turn (step 5), so it lives in `src/tests/kalah`.

## Approaches considered

1. **A count on a piece** (`Piece.count`): least change, but one kind per place, so no clearing
   with two players' warriors, and bags still special.
2. **One holder store, existing blocks widened (chosen):** any game's pools, hands and piles of
   one kind are the same thing; blocks are reused; the cost is a second store to keep in step
   with `pieces` in views, tests and saves.
3. **New effects** (`SOW`, `DRAW`, `TRANSFER`): quick, but each serves one idea. Refused by the
   pass's rule.

## Left to the builder

Names (`bags`, `holder`, `FOLLOW`, `HELD`, `skipping`, `LAST`, `ALL`) are placeholders Frank may
rename. Kalah's end and score need the smallest general block that counts a player's stones by
zone; the builder picks it and records it as a ruling. Anything else a Kalah turn needs that the
blocks above do not give is a finding to bring back, not a rule to add.
