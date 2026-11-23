# The hand, and where removed pieces go (step 4c-1)

Step 4c was "places that hold many pieces". Frank's go on 2026-11-23 split it in two, as 4 was
split before: **4c-1** (this document) replaces the three hand rules the route doc names for 4c;
**4c-2**, a place that holds a bag of pieces (capacity above 1, supplies and pools, Root's
clearings), gets its own design after 4c-1 is merged, because it changes what a board place is
and must be timed against the grid games.

## Intent

The hand is a special case wired into the engine: a game-wide switch (`hand_enabled`), a table on
each piece (`demotes_to`), and a rule that every capture, by any effect, puts the captured piece in
the capturer's hand. Frank prefers the spec to say where a removed piece goes, on the effect that
removes it, and does not mind the repetition. The engine then has no rule about captures.

## Decided with Frank (2026-11-23)

- **Bags only in 4c** (no ordered piles): a place holds counts per (player, code). Decks, which
  need order, shuffling and hidden contents, come with cards in step 7.
- **Counters stay plain numbers.** Anything that moves between players is a piece in a bag; a
  score or a track is a number. Slice 4's counters do not change.
- **Where a removed piece goes is written on the effect** (his option B), not as a game-level
  rule.
- **`hand` is its own top-level key**, `{ "enabled": true }`, with `enabled` defaulting to false,
  so that more about the hand can live there later. It replaces `hand_enabled`.

## Goals, non-goals

Goals: `hand` replaces `hand_enabled`; `removed_to` on `MOVE` and `REMOVE` replaces the implicit
capture-to-hand; `captured_as` replaces `demotes_to`; Shogi and Crazyhouse play exactly as now;
the old forms are refused with a message naming the new one.

Non-goals: a place holding several pieces, supplies and pools (4c-2); starting contents of a hand
(Nine Men's Morris wants them: a later field of `hand`, since the key is open); hidden hands;
`OWNER` as a destination (below).

Anti-goal: a hand-enabled game that silently stops filling hands. A game with `hand.enabled` and
no `removed_to` anywhere is allowed (a game that only drops from nothing is not a game today), but
`removed_to` without `hand.enabled` is refused when the spec is read.

## The design

**Spec.**

```json
"hand": { "enabled": true }
{ "effect": "MOVE", "from": {"place": "START"}, "to": {"place": "TARGET"},
  "removed_to": { "hand": "MOVER" } }
{ "effect": "REMOVE", "at": {"place": "TARGET"}, "removed_to": { "hand": "MOVER" } }
{ "code": "DRAGON", "captured_as": "ROOK", ... }
```

- `removed_to` is allowed on `MOVE` (it names where the piece removed from `to` goes) and
  `REMOVE`. Its only value in 4c-1 is `{ "hand": "MOVER" }`: the piece goes to the hand of the
  player the way is played for. `{ "hand": "OWNER" }` (Root: a removed warrior returns to its own
  player) is refused with a message that says it comes with 4c-2. *Claude's call.*
- Only a piece of another player enters the mover's hand, as today: a removed piece of the
  mover's own is simply removed. *Claude's call, to keep Shogi and Crazyhouse identical.*
- The code that arrives is the piece's `captured_as` (default: its own code), the field
  `demotes_to` was. Frank's note kept the per-piece table, since it cannot sit in each effect.
- `hand_enabled` and `demotes_to` are refused when written, each naming its replacement (the
  `old_forms` pattern).

**Engine.** A way records what it puts in the mover's hand (`Way.to_hand`, codes already
resolved through `captured_as`), next to `from_hand`. Playing a way applies both; the
recomputation `take_into_hand` does today (walking the changes against the board) goes away, and
Shogi and Crazyhouse get faster for it. The saved state does not change: `state.hand` is already
a per-player bag (player → code → count), so old saves load as they are. It becomes one place
among others in 4c-2.

**Catalogue.** Shogi and Crazyhouse get the key and `removed_to` on every effect that removes an
enemy piece, written by script; their move counts and hand tests pin them.

## Approaches considered

A. A game-level rule ("removed pieces go to the capturer's hand"): least to write, no repetition.
Frank chose B.
B. Written on each effect (chosen): the engine has no capture rule, Root's and Shogi's cases are
one mechanism, and a game may send only some removals to the hand.
C. Both (rule with an effect override): two ways to say it, so rejected.

## Left to the builder

The name `removed_to` and the value shape are placeholders Frank may rename in review. The
order of two captures in one way (an en passant plus a normal capture cannot meet in one way
today) is the order the effects run in.

## As built (2026-11-23)

Built as designed. `Way.to_hand` carries the codes; `take_into_hand` is gone; the saved state is
unchanged. One behaviour differs from the old implicit rule, outside the catalogue: the old rule
also put an enemy piece in the hand when an effect replaced it with the mover's (`SET_OWNER`, or
`PLACE` onto an enemy). Now only `MOVE` and `REMOVE` send pieces to a hand, and only with
`removed_to`; no shipped spec used the old behaviour. Speed against `main`: chess -0.4%, Shogi
-0.3%, Go 0%. A differential run of 100 seeded random games of Shogi and Crazyhouse against
`main` gave identical states after every answer.
