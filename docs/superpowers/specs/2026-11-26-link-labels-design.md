# Link labels on maps, and the full Kalah

Item 2 of `2026-11-23-remaining-work.md`. A short design: a small change.

## Intent

A spec can name its own link labels on a map, so a rule can follow "the pit across". The proof is
the full Kalah, with its capture: a last stone that lands in an empty pit of the mover's own side,
with stones in the pit across, takes them and the last stone into the mover's store. Kalah then
moves from the engine's test folder into the catalogue.

## What is true today

- A map has six fixed labels (`next`, `ends`, `paths`, `corners`, `areas`, `sides`), listed in
  `MAP_LABELS` and read in four places: the map compile (`specs/game/map.rs`), the spec validation
  (`specs/game/game.rs`: steps, `AROUND`, `FOLLOW`), the board's link tables
  (`logic/places/board/map.rs`) and the label table of the game.
- A written link goes two ways (`["a", "b"]`) or one way (`{ "from", "to" }`), both under `next`.
- `AROUND` already takes any map label as `through`, and `MOVE` takes `"count": "ALL"`. So with a
  label for the pit across, the capture needs no new test or effect: a `COUNT` on `LAST` (one stone:
  it was empty), `IN_ZONE` of `LAST` in the mover's pits, a `COUNT` over the pit across, and a
  `FOR_EACH` over `AROUND of LAST through "across"` that moves all its stones to the store.

## Goals

- A written link may carry a `label`. A one-way link: `{ "from": "a3", "to": "b4", "label":
  "across" }`. A two-way link, new: `{ "between": ["a3", "b4"], "label": "across" }`. The plain
  forms keep meaning `next`.
- A label a map writes is known to the map: steps, `AROUND` and `FOLLOW` may name it; a label no
  map has is still refused (`UnknownLabel`), now against the map's labels.
- Refused when read, with `InvalidMap`: an empty label or one over 64 bytes; a label that is one of
  the five the engine works out itself (`ends`, `paths`, `corners`, `areas`, `sides`), since a
  written link cannot add to those. `next` stays writable, as it is today.
- A labelled link joins two points, as links do. Its label leads wherever the written links lead:
  from one place, to several. `FOLLOW` still refuses a label that leads to several from one.
- The web view draws only links under `next`; a labelled link is no line on the board.
- Kalah (`kalah.json`, a catalogue spec) with the capture, `across` links pairing a1-b6 … a6-b1, a
  count row in `catalogue_counts`, and its catalogue page if the map skin of Pong hau k'i carries
  over; if not, the page is left out and reported.

## Non-goals and anti-goals

- No per-player links and no sliding along links (`repeat` on maps): no game needs them yet. They
  go back on the roadmap.
- No change to what any existing spec means; every catalogue, book and fixture spec reads as before.
- A labelled link that silently becomes a drawn line, or a misspelt label that silently means
  nothing, is the failure to avoid.

## Approaches

1. **A `label` on links, labels known per map. Chosen.** Smallest; the capture is then ordinary
   blocks.
2. A new `PAIRS` or `OPPOSITE` block for Kalah's capture. Rejected: a block only Kalah could use.
3. Per-player links now. Rejected for scope (Frank: labels only for now).

## The builder's part

`MapSpec.links` becomes keyed by label name (`String`) rather than the fixed array; the board's
tables follow the map's labels. Test names; message wording; whether a spec that serializes a map
back writes labelled links in the object form (it must round-trip).

## Calls I made (for you to check)

- The two-way labelled form is `{ "between": [a, b], "label": L }`, since `["a","b"]` has no room
  for a label and `from`/`to` means one way.
- `next` stays writable as a label; the five derived labels are refused.
- The capture is written with the existing blocks, not a new one.

## After this

The closing item of the run, by Frank: versioning specs (`1` is what exists today). It gets its
own design once link labels are merged.
