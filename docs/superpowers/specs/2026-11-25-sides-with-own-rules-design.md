# Sides with their own rules (step 8a)

Step 8 of `2026-11-21-engine-generalisation-design.md`, first half; the second half, answers out of
turn, is its own design (8b). Written before building, so every call below is **Claude's** unless it
says otherwise; Frank may overrule any of them.

## Intent

An asymmetric game: two sides that do not have the same pieces, reserves, obligations or way to win.
Most of this already works, and the work is to find the little that does not and add only that.

## What already works (found by reading the engine, not by building)

- **Different moves:** a piece's `move` has `players`; a piece is a side's by who it starts as.
- **Different actions:** an action has `players` (the Flying Knight's White `MOVE`, Black
  `DOUBLE_MOVE`), and `every_action` has `for`, which names actions, so a rule that runs for one
  side's action only is written by giving that side its own action.
- **Different pace:** `turns.order` may repeat a name (`["RAIDER", "DEFENDER", "RAIDER"]`).
- **Different start:** `starting_positions` are per player.
- **Different goals:** an end check has `players` (Flying Knight), and `POINTS` has `to`.
- **A stage:** only the player whose way ran `STAGE` is in it, so it is that side's already.

So "extend `players` to pieces, places, actions, stages and end checks" (the sketch) is mostly done:
pieces and places have no rules of their own to scope, and the rest is above.

## The gaps

Two, found by asking what an asymmetric game would have to write that it cannot:

1. **A rule that narrows the ways for one side.** `must` (the forced capture) is a list of tests
   with no way to say whose obligation it is: Checkers' rule binds both sides. A side-only
   obligation cannot be written without giving that side an action of its own, and a side whose
   pieces move by many actions would need the obligation copied into each. The same holds for any
   `every_action` entry that should run for a side, whatever the action. **Call: one new test,
   `PLAYER`,** `{ "test": "PLAYER", "is": ["DEFENDER"] }`, true when the player being judged is
   listed (the mover in a way, the `as` player in an end check). A side-only obligation is then
   `ANY [ NOT PLAYER is [DEFENDER], <the obligation> ]`: for the other side the first branch holds
   for every way, so `must` narrows nothing. The same test works in `every_action`, a stage's
   `only`, and an `if` inside an action (a shared action that does one thing for each side), so it
   is the one primitive rather than a `players` field on each. No stored spec changes.
   *Not chosen:* a `players` field on every `must` and `every_action` entry (changes the shape of
   `must`, which is a bare list of tests, and covers only those two places).
2. **A different reserve for each side.** A bag's `start` is the same for every owner: the same
   hand for both players. A game where one side drops its pieces from a reserve and the other has
   none cannot be written. **Call: `start` may be written as a map from player to the starting
   pieces** (`"start": { "RAIDER": { "R": 12 }, "DEFENDER": {} }`), as well as the form it has now
   (the same for each owner). A player left out has an empty bag. Only a bag each player has may use
   the map form; the plain form is unchanged, so no stored spec changes.

Not done, because no game needs it yet: scores with a term for one side only (`POINTS` already has
`to`), a different `capacity` per player, a side-only zone.

## The proof: Siege

A small original game, `Siege`, in the tests and not in the catalogue (like the dice race and
Draft). A 7x7 board with the keep in the middle.

- **DEFENDER:** a `LORD` on the middle square and four `GUARD`s around it. Both step one place
  in any of the four directions, and either may take a raider by stepping onto it. The defender
  is **bound to capture**: if some way captures, only those are offered. (A `PLAYER` inside `must`.)
- **RAIDER:** no pieces at the start and a reserve of twelve `RAIDER`s. A turn **puts one on an
  empty edge square**, or **steps** one on the board one place, and takes what it steps onto. No
  obligation. (The reserve is the per-player `start`.)
- **Turns:** `RAIDER`, `DEFENDER`, `RAIDER`... as `order` says.
- **Ends:** the raiders win when the lord is taken. The defender wins when the raiders have no
  piece on the board and none in the reserve. A side with no way to play, as in any game here,
  draws (an end check with `STUCK`, judged for each side by `players`).

It needs both gaps and the rest (each side with its own actions, a different reserve, a different
obligation, a different win) and is proven three ways: played to the end by each side winning, with
every snapshot over the same engine; a test per gap that fails without it (the forced capture
binds the defender and not the raider; the raider's reserve is twelve and the defender's none); and
a seeded random play of many games that never reaches a state the rules forbid.

## Approaches considered

1. **A `players` field on every list that lacks it (chosen against).** Four fields to document, and
   `must`'s shape breaks.
2. **One `PLAYER` test (chosen).** One thing to learn, works wherever a test works.
3. **Whole rule sets per side** (`sides: { WHITE: { ... }, BLACK: { ... } }`). The honest shape of
   "sides with their own rules", but a second way to write what the spec already says, with every
   check duplicated. Refused: the pass's rule is that nothing is written for one game.

## Left to the builder

The names (`PLAYER`, `is`) and where the per-player `start` is read when the spec is parsed (the
map form must be refused for a shared bag, with an unknown player, and with a count of 0, like the
plain form). Siege's board size and piece counts, which the tests may tune so a game ends.
