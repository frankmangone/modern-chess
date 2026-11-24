# Dice and shuffles (step 6)

Step 6 of `2026-11-21-engine-generalisation-design.md`, in the order of
`2026-11-23-remaining-work.md`. Written and built in one session, so every call below is
**Claude's** unless it says otherwise; Frank may overrule any of them.

## Intent

Chance, kept fair and replayable. A spec can say "roll two dice" and "shuffle the deck" and the
engine does it, with no code outside the spec and no way for a player to know the next result from
anything they can see. The rule of the pass holds: nothing here is written for one game. It adds one
source of chance and two blocks that use it, and widens what a bag can be.

## Goals, non-goals, anti-goals

Goals: a seed that lives with the match on the server; a `ROLL` effect that puts random faces in a
holder; ordered piles (a bag whose pieces have an order) and a `SHUFFLE` effect that reorders one;
drawing from the top of a pile with the blocks that exist; every result recorded in the history; a
match that replays exactly from its seed; a small original dice race that needs all this.

Non-goals: hidden information (step 7: who may see a pile's contents or order stays as it is, so a
pile is visible like any bag, in counts and without its order); a card game (step 7's proof);
dice that stay individual (a roll adds pieces to a holder, it does not give each die its own
place); weighted randomness other than by repeating a face; random choices of the engine for the
player ("pick a random move"); a random choice among ways.

Anti-goals: any change to how an existing game plays, to a stored spec or to a stored state (a game
that never rolls has no seed and no new key in its state); a roll that can be known before the
player commits to it; a randomness source inside the engine (it stays a pure function of what the
state holds); a new dependency.

## Where chance comes from

**The seed.** `GameState` gains `seed` and `draws`, both optional in the JSON and left out when
unset: the seed is a 64-bit number the host supplies when a match is created, `draws` counts the
random numbers the match has used so far. A game whose rules never roll or shuffle gets no seed
and no `draws`, so every stored state loads and saves as it did.

**The generator is a function, not a stream.** The *n*th random number of a match is
`mix(seed, n)`, SplitMix64's finalizer over `seed + (n + 1) * 0x9E3779B97F4A7C15`. There is no
generator state to keep in step: the match's position in the sequence is the single number
`draws`. A way worked out for the player to move reads draws `draws`, `draws + 1`, ... and a way
that is played adds what it used to `draws`. A number in `0..n` is `(x * n) >> 64` (a bias below
2^-40 for any `n` a spec can write, which no game can see).

**Why this works with the engine as it is.** Ways are worked out once per turn, before the player
answers, and replayed on every request by resuming the saved state. A function of `(seed, draws)`
gives the same ways every time, so nothing about resuming, cancelling, `shown_pieces` or the
turn's look-ahead changes: a way that is not played leaves `draws` where it was, and a way played
moves it by exactly what it used.

**Who supplies the seed.**

- The game server, when it creates a match (`GameInstance::new_lobby`): 64 bits of a fresh
  `Uuid::new_v4()` (the `uuid` crate is already a dependency and draws from the operating system).
  A rematch is a new match and so has a new seed.
- The browser's "Try it" board: the wasm `Board` reads `Math.random()` itself, so no page changes.
  `reset()` keeps playing the same game from a new seed derived from the old one.
- Everything else that builds a `Session` or a `Game` without one (tests, the examples) has no seed,
  and a game that rolls plays the sequence of seed 0. That is documented, and fine: nothing real
  builds a match that way.

The seed never leaves the server: a client is sent the *view*, which has no seed, and the state
JSON stays in the database. (Step 7 owns what else a view may not carry.)

## The blocks

### `ROLL`

```json
{ "effect": "ROLL", "at": { "bag": "TRAY" }, "dice": 2,
  "faces": [ { "PIP": 1 }, { "PIP": 2 }, { "PIP": 3 }, { "PIP": 4 }, { "PIP": 5 }, { "PIP": 6 } ] }
```

Each die picks one of `faces` with equal chance, and the pieces of that face go into the holder
`at`. A **face is a set of pieces** (`{ "CODE": n }`), so one idea covers a die of pips (a face is
`n` pieces of `PIP`, and "the number rolled" is a count of `PIP`s in the tray, which the blocks
already read), a die of symbols (`{ "SWORD": 1 }`) and a die of mixed results. To weight a face,
list it more than once.

- `at` is a bag or a map place that holds many, as `PLACE` takes. The pieces enter it as `PLACE`
  puts them (the mover's in a player's bag, nobody's in a shared one). A roll that would pass the
  holder's capacity is not offered, like any effect that does not fit.
- `dice` is a whole number above 0, or `{ "count": { ... } }` (the number of dice can be read from
  the position). Not `ALL`. Default 1.
- A roll *adds* to the holder; clearing the last roll first is a `REMOVE` with `count: ALL`.
- Read-time refusals, as for every other block: `at` is not a holder, `faces` is empty or has more
  than 1000 faces, a face is empty or has a count of 0, a face names a code the game does not have,
  `dice` is `ALL` or 0.

### Ordered piles and `SHUFFLE`

A bag may be declared `"ordered": true`. It keeps its pieces in order, top first, instead of
counts. Everything that works on a bag keeps working, with these differences:

- a piece that enters goes on **top**;
- pieces that leave are taken from the top, in order (so `MOVE` from a pile with `count: 3` draws
  the three top pieces; a hand is another bag, so "draw" is a `MOVE` from the pile to the hand);
- `what` (a filter) is not allowed on a pile, and a `HELD` choice on one is refused: a pile has no
  "pick a card from it";
- the view lists a pile's counts per code, like any bag, and not its order, until step 7 says who
  may see it.

`{ "effect": "SHUFFLE", "at": { "bag": "DECK" } }` puts the pile's pieces in a random order. Only an
ordered bag can be shuffled (refused otherwise), and a pile of one or none is unchanged but still
uses no draw.

Putting a discard pile back is `MOVE` of `ALL` from it to the deck, then `SHUFFLE`: the blocks
compose.

### What the history records

`MoveRecord` gains `rolled` and `shuffled`, both left out when empty and defaulting to empty, so a
saved history loads. `rolled` lists, for each die in order, the holder and the face it landed on.
`shuffled` lists the piles that were shuffled and **not their new order**: an order is exactly what
step 7 will hide, and the seed already makes the match replay exactly.

### A roll cannot be peeked at

An action's ways are all known before the player picks one, and a way that fails is not offered.
So a `choose` *after* a `ROLL` or a `SHUFFLE` in the same action would show the player the result in
the options it offers, and a bare `test` after one would show it by its absence (the way vanishes
when it fails). Both are refused when the spec is read: **a choice or a bare test may not follow a
`ROLL` or a `SHUFFLE` in an action** (nested `if`s and `FOR_EACH`es count; an `if` is fine, it removes
no way). Choices and tests come first, then the chance, then the effects. A game that wants "roll,
then choose what to do with it" rolls in one action and hands on with `STAGE`, and the choice is
made in the stage, after the roll is played.

What stays the author's to keep clean, because only running the rules shows it: an effect after a
roll that does not fit (a capacity passed, a `FOLLOW` over a dead end, more taken than a holder has),
a test in `every_action` or `must`, and an end check all read the result, and a way they remove is
not offered. This is written in `SPEC_FORMAT.md`. Step 7 will decide whether the engine should
close these too.

## The proof: a dice race

`Dice race`, in `src/tests/dice_race` and not in the catalogue (the web pages and seed entry for a
new catalogue game are their own piece of work; the Flying Knight waits for the same). Two runners
on a track of 18 places in a line, `t0` to `t17`; the finish is the last six. A turn: pick your
runner, roll one die (faces of 1 to 6 pips) into the shared tray, run that many places, clear the
tray. The first runner on a finish place wins. It uses `ROLL`, the tray as a shared bag, `FOLLOW` and
`FOR_EACH`, a count of pips as a number, `END` checks: no rule for dice races.

It is proven three ways: a play to the end, every roll in the history and in range; the same seed
replays the same game and another seed does not; and a run of many seeds shows each face coming up
at about one sixth (a statistical check with a loose, fixed bound).

The shuffle is proven on a small deck game written only for its test: a pile of ten, shuffled, drawn
into a hand; the same seed gives the same order, saved and resumed mid-game gives the same draw.

## Approaches considered

1. **A stateful generator in the state** (xoshiro or similar: the state is four words). Smaller
   to explain, but resuming means saving and restoring that state at exactly the right moment,
   and the ways computed ahead of a choice would have to avoid advancing it. Refused.
2. **A function of `(seed, draws)` (chosen).** One counter, ways stay pure, no dependency.
3. **Record every result and replay the record instead of a seed.** Fair, but a client could not
   be stopped from knowing a result recorded ahead, and the record would need to be pre-generated
   or kept open. Refused; the history still records results, after the fact.
4. **Roll when a turn starts, before any action.** Hides the peek problem but is a special case in
   the turn structure, and a game that rolls only on some actions could not say so. Refused for the
   rule above, which keeps the roll a normal effect.

## Left to the builder

The names (`ROLL`, `SHUFFLE`, `faces`, `dice`, `ordered`, `rolled`, `shuffled`, `seed`, `draws`)
are placeholders Frank may rename. How a pile's order is stored (it must keep saved states loading
and must not change the JSON of a bag that is not ordered). The statistical bound in the
distribution test.

## As built (2026-11-27)

Built on `feature/dice-and-shuffles`; as designed, with these differences and findings.

- **`ROLL` and `SHUFFLE` take a bag, not a place** (`at` is `{ "bag": ..., "of": ... }`). A map
  holder place cannot be rolled into or shuffled yet; nothing needed it, and "bag" made the read-time
  checks exact.
- **A pile is stored by replacement.** A change to a pile is written into the way as the pile's whole
  new contents (`HolderChange::replacing`), not as counts added and taken, so order survives the
  overlay the tests read and the way's own bookkeeping. A bag that is not ordered is untouched: its
  JSON, and the counts-and-deltas path, did not change. A pile's runs of the same owner and code
  join, so a pile saves as the same list a bag does.
- **No peeking** was widened from questions to bare tests (see above): a test that fails removes the
  way, which shows the result as plainly as a question does.
- **`MoveRecord.rolled` and `shuffled`**, with `RolledRecord { bag, face }`. Neither is in a view.
- **Seeds.** `Game::set_seed` (a no-op for a game that never rolls, so such a game's stored state
  and a stored chess match are byte for byte as before); `Session::new_with_seed`, and `reset()`
  deals a new seed worked out from the old; the game crate seeds a new match from a fresh UUID's
  first 64 bits (`GameInstance::new_lobby`); the wasm `Board` draws from `Math.random()` itself
  (the clock stands in off the browser, so the crate's native tests run).
- **The dice race lives in the engine's tests** (`src/tests/dice_race`) and, as a fixture, in
  `crates/session/tests/fixtures/dice_race.json`, which the session and game crates' tests play. It
  is not in the catalogue.
- **A finding worth Frank's eye.** The runner's walk (put on each place, taken off the last) needed no
  new block, but "move a piece along a walk" is awkward: `MOVE` from `START` only works on the first
  place, since `START` does not follow the piece. A `MOVE` that follows the picked piece, or a
  region's last place as a place, would make it direct. Left as a later call.
- **What was not done:** a book chapter (the reference, `SPEC_FORMAT.md`, and the known-limitations
  page are updated); a catalogue entry with a page and an image for the dice race; individual dice
  (each in its own place); putting a card under a pile (a piece goes on top).
- **Verification.** The engine suite (795 tests; new: 27 chance tests, 9 of the dice race, 6 of the generator and 2 of the pile operations),
  the session tests (a new `chance.rs`, except `boards.rs`, which needs the book's derived boards:
  `apps/docs/build.sh`), the game crate against the dev Postgres (a new domain test and an HTTP
  match with dice), `make lint-panics`, and `cargo check --target wasm32-unknown-unknown` for the
  wasm build. `catalog.rs::embedded_seed_files_publish_the_launch_games` fails on `main` too (it does
  not list the Flying Knight and Kalah); not touched here.
