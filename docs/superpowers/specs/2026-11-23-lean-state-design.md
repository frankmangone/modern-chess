# Lean state (slice 5 of the generality pass)

Slice 5 of `2026-11-22-rule-blocks-design.md`. Slices 1–4 made the rules blocks. This slice makes
the state they run on cheap to read and to carry: names become small ids, a candidate move stops
copying pieces, and a position's attack sets are worked out once.

Frank chose the approach on 2026-11-23 ("A sounds good"), asked that copying and cloning be cut to a
bare minimum, and agreed that attack sets are cached without changing which moves are legal.

## Why

A profile of the timing run (chess, Shogi and Go to depth 3, 3 and 2, on `main` at `6ccb435`) puts
about 32% of its time in `malloc` and `free`, 13% in SipHash, 15% in `memcmp`, `memmove` and
`memset`, and 2% in `String::clone`. A `Position` is a `String`, a `Piece` holds two `String`s and
a flag map, and the runner's working state is cloned at every branch. A candidate move records the
pieces it removes (`removed: Vec<Piece>`) although the only readers need three small facts about
each.

## Goals

- Names that the hot path compares and hashes (positions, piece codes, players, flag names) are
  `Copy` ids. Names stay at the edges: specs, saved games, the session view, the game service.
- A candidate move copies no `Piece`: what it removed is recorded as small `Copy` snapshots.
- Each position's attack sets are built once and lent to every test and check that asks.
- Timing run: faster on chess and Shogi (and not slower on Go), with identical move counts and the
  catalogue counts unchanged. Saved games still load and save byte for byte.

## Non-goals

- Changing which moves are legal. The two attack-set findings carried here (a slide over pieces
  threatens fewer squares than it reaches; a zone test on a move does not limit its threats) stay
  as they are, by Frank's call. They go to a later task of their own, with rulings from Frank.
- Hands and supplies as places (step 4c); turn stages (step 5, with the Flying Knight).
- Any change to the spec format or to what the client protocol sends.

## Anti-goals

- A name that behaves differently from its text: an id compares, sorts, prints and saves exactly
  as the string it stands for.
- A speed claim without the timing run beside it.

## Approaches

- **A. Borrow first, intern second (chosen by Frank).** Measure; record snapshots instead of
  pieces; intern names; cache attack sets. Each step is timed alone, so a step that does not pay
  is dropped.
- **B. Interning only.** Cheaper clones, but the clones stay. Ruled out.
- **C. Borrow and snapshot only.** Smallest change; strings stay in the state, so hashing and
  comparing stay slow. Ruled out.

## The design

### Names as ids

A global interner (leaked, append-only, behind a lock) maps each distinct name to a `u32`, and an id
back to its `&'static str`. Two types sit on it:

- `Position` keeps its API (`new`, `from_coordinates`, `as_str`, `coordinates`, `From<&str>`,
  `Display`, `Ord`, serde) but holds an id, is `Copy`, and compares and hashes as the id. Its order
  and its saved form are exactly today's (coordinates first, then names). Code that clones one
  keeps compiling.
- `Symbol` is the same for piece codes, players and flag names: `Copy`, `Deref<Target = str>`,
  equal to `str`, `&str` and `String`, hashed by id, saved as its text.

The names the interner holds are the game's own: places of the board, codes, players, flags. A
board's nodes are interned when it is built, so the hot path never mints one. A name met while
playing (a spec string compared against a piece) is looked up, never added, where the code can say
so.

### Snapshots, not pieces

`Way.removed` becomes `Vec<Removal>` with `Removal { code: Symbol, player: Symbol, moved: bool }`
(`Copy`). `REMOVED`, the captured counts and the history read those three facts and nothing else.
`Way.moved` stays a list of node ids. A `BoardChange` that sets a piece still carries the piece it
sets (it must be built to be played); with ids and an empty flag map, that is a plain copy of a few
words and no allocation.

### One hash for the state

Maps keyed by `Position` or `Symbol` use a fixed fast hasher (a multiply-and-fold over the id), in
one place (`type IdMap`/`IdSet`), not SipHash. Order of iteration is never part of the contract
(maps are sorted where order shows), so changing the hasher changes no output.

### Attack sets once

The cache is per position, held with the game state: built on first ask, dropped when a way is
played. It is exactly what `calculate_threats_with` returns today (including the two findings above,
unchanged), so every answer is the same; only the number of times it is worked out changes.
`ATTACKED` stays lazy where it is lazy today (it stops at the first attacker) and uses the cache only
where it already builds the whole set.

## Checked

- `catalogue_counts` and the speed example's `moves=` unchanged for every game; the whole test
  suite green; the saved-state tests (a state saved before this loads and saves unchanged).
- New tests: ids compare, order, hash, print and serialise as their text; a removed piece's
  snapshot is what `REMOVED` reads (code, owner, has-moved); the attack-set cache gives the same
  threats as the direct call on every position of a short walk of every catalogue game.
- Timing run, release, alternating with `main`, after each task: chess and Shogi faster, Go not
  slower, medians over three alternations.

## Left to the builder

Type and file names; where the cache lives; the hasher's exact form; the order of commits; which
clone sites the profile says are worth removing beyond the ones named here.

## As built (2026-11-23)

What the build changed from the sketch above, each a call recorded in the ledger:

- **Names are pointers, not numbers.** An interned name points at its leaked text entry
  (`&'static Entry`), so reading one takes no lock and comparing or hashing it is one word. The API
  is the one designed (`Position` and `Symbol`, `Copy`, ordered, printed and saved as text).
- **No attack-set cache.** `ATTACKED` now visits a piece's threats one by one and stops at the first
  hit (`visit_threats`), building no sets, so there is nothing to store or to invalidate. The set
  version (`calculate_threats_with`) is the same walk collecting a set; a test compares them on every
  catalogue game.
- **A borrowed view of the position (`PiecesView`).** A way's tests see the position the turn
  started from with the way's changes laid over it (and the piece held in the hand), instead of a
  copy of the map of pieces made for each way. The repetition test counts timed flags down while it
  builds its key instead of copying the pieces.
- **`Removal` snapshots** (`code`, `player`, `moved`, flag names) replace copied pieces in a way.
- **One-word hasher and small player lists.** Maps keyed by names use `IdMap`/`IdSet`; per-player
  rule tables are `PlayerMap`, a short list searched in order.

Timing run against `main` (`6ccb435`), release, three alternations, medians: chess 93.9 → 27.9 ms,
Shogi 265.0 → 61.7 ms, Go 10.5 → 6.7 ms; move counts identical.
