# Hidden information (step 7)

Step 7 of `2026-11-21-engine-generalisation-design.md`, after dice and shuffles
(`2026-11-27-dice-and-shuffles-design.md`, merged as #89). A design only: this document and its
slices are the whole of the first pull request. The calls are Claude's unless marked; Frank ruled on
the open ones on 2026-11-28 and they are listed at the end under "Settled by Frank".

## Intent

A game can keep things from some of its players: a hand only its owner sees, a deck nobody sees, a
count everyone sees but whose contents nobody does. What a player may not see never reaches them:
not in a view, not in a history, not in an error, not by a legal move that exists only because of it.
The pass's rule holds: no rule for any one game, only a declaration on what already exists (bags)
and a view that depends on who is looking.

The leak is the risk, not the feature. A secret that is hidden in the board and shown by the
choices, the score or the log is not hidden. So this design is mostly an inventory of the paths
information takes today, and one rule for each.

## Goals, non-goals, anti-goals

Goals: a bag says who sees what it keeps and whether its count shows; the session builds a view for
one viewer (a seat, a spectator, or everyone); the game server sends each seat only its view; the
history and the derived numbers (material, score) are redacted the same way; reading what a player
may not see is refused when the spec is read; a card-drafting game proves it, played over HTTP, with
a test that searches the opponent's response for the cards.

Non-goals: hidden *board* squares (fog of war, face-down tiles on the board: a place on a grid or map
stays public, and a map holder place stays public); hidden *moves* (a face-down play); peeking at a
pile (a rule that lets a player look at the top card is a later block); hiding information in
the stored state (the database is trusted: the seed and every hand are in it); anything that
authenticates the viewer (the token already says who calls); a second engine for the browser.

Anti-goals: any change to a game that declares nothing hidden (its views, its history and its
state are byte for byte as now); a view built by deleting fields from a full one at the last
moment in the game server (the engine's session builds it, and the game server cannot ask for the
full one by mistake); a secret that depends on the author remembering to hide it in several places.

## What is true today (the paths information takes)

Read from the code at `7e2e2fc`:

| Path | What it carries | Hidden-information risk |
|------|-----------------|-------------------------|
| `View.bags` (`session/src/view.rs`) | every bag's contents, counts per code | the hand and any deck, to everyone |
| `View.pieces` | pieces on places, and counts in holder places | a map holder's contents (kept public) |
| `View.material` | worth of the board and **every owner's bags** | a number computed from a hand |
| `View.score` | the score terms as if the game ended now | a term that read a hidden place |
| `View.choice` | what the mover may answer next, **including `held`** (their hand) | emptied for everyone but the mover by the game server; shows the mover a hand they may not own (`HELD` in `ENEMY`'s bag) |
| `View.captured` | pieces lost per player | public by nature (lost from the board) |
| `GameState.history` (`MoveRecord`) | the answers, `piece_code`, and now `rolled` and `shuffled` | not sent today (no history endpoint); a roll into a hidden bag would carry its faces |
| `GameState` JSON in `game_instances.state` | everything, seed included | stored server-side only; the snapshot never includes it |
| `Snapshot` (`game/src/http/play.rs`) | `summary` + `view` + `outcome` + `you` | built from `session.view()`, the full view, for every caller |
| The ways of a turn | computed on the full state | an option that exists only because of hidden contents tells the mover about them |
| `{"changed": false}` and the revision | a number that moves each command | nothing: it is the same for everyone |
| An error's message | `"that is not a legal move"`, fixed | nothing: the engine's reason goes to the log |
| The wasm `Board` ("Try it", the book) | the full game in the browser | a hot seat: one person plays every side, so no secret exists to keep |

The browser board in a *match* is a `RemoteBoard` drawn from the server's view; the engine never
runs there. So "the wasm engine never receives the rest" holds by construction as long as the view
is the only thing sent. What this step must make true is that the view is built per viewer.

## The design

### Declaring what is hidden

Three optional fields on a bag:

```json
{ "name": "HAND", "for": "EACH_PLAYER", "visible": "OWNER" },
{ "name": "DECK", "ordered": true, "visible": "NONE", "shows_count": true, "strict": true },
{ "name": "ROW" }
```

- `visible`: who sees the **contents**. `"ALL"` (the default: everyone, as every bag today),
  `"OWNER"` (the player a bag belongs to; a shared bag belongs to nobody, so `OWNER` on one is
  refused), or `"NONE"` (nobody until the game is over).
- `shows_count`: whether everyone sees **how many** pieces it keeps when they cannot see which.
  Default `true` (a hand's size and a deck's height are public in nearly every game). Meaningless,
  and refused, with `visible: "ALL"`.
- `strict`: opts the bag into the read-time rule 5 below, which stops the rules from reading what
  the mover may not see. Default `false`. Meaningless, and refused, with `visible: "ALL"`.

A map place that holds many has neither: it stays public (non-goal above), and the checker refuses
the fields there, so a game never believes a clearing is secret.

### Who is looking

`Viewer` is `Player(name)`, `Spectator`, or `Everyone`. `Everyone` is today's view: the hot seat of
"Try it", the book and the tests. A seat sees what `visible` says for its own bags and for the
public ones; a spectator sees only the public. **When the game is over, every bag is shown to
everyone**: the cards were hidden to protect the play, and the end ends the play.

### The view per viewer

`Session::view_for(&Viewer) -> View`; `Session::view()` is `view_for(&Viewer::Everyone)`. In
`View.bags`, a bag the viewer may not see has `"hidden": true` and `pieces` empty, and a `count`
when `shows_count` is on. Nothing else about the shape changes, and a view of a game with nothing
hidden is exactly what it is now (the new fields are left out).

The game server builds a snapshot from `view_for` with the caller's seat (or `Spectator`, signed in
or not), and a unit test of the server's handlers fails if a snapshot is ever built from `view()`.

### Closing each path

1. **Bags and counts:** the view above.
2. **`material`:** a number that reads a bag the viewer may not see is not shown (the key is left
   empty) until the game is over. *Rule for every derived number:* **it is shown only when it reads
   nothing the viewer may not see.** Today only `material` reads bags.
3. **`score`:** no score term reads a bag today. The rule above applies the day one does: a new term
   over a bag is withheld, for every viewer, until the game is over. The proof game needs one, so
   Slice C adds the term (`BAGGED`, the pieces a player's bag holds).
4. **`choice`:** only the mover has one, and only what the mover may see. **A `HELD` choice may only
   name a bag whose contents the mover can see** (`ALL`, or the mover's own with `OWNER`): choosing
   a card out of the opponent's hand would show the options. Refused when the spec is read.
5. **The ways themselves.** An option that exists only because of what the mover may not see
   leaks it: "take the opponent's red card" is refused when the card is not there, and a way offered
   only `if` the deck holds an ace tells the mover about the deck. This is the one path the engine
   cannot close by building a different view, so it is **opt-in per bag, with `strict: true`**, and
   then the checker enforces: **in an action, a strict bag the mover may not see can only be named as
   the source or target of `MOVE`, `REMOVE`, `PLACE`, `ROLL` or `SHUFFLE`, with no `what`, and is
   never read by a test, a `COUNT`, a `count` region, `FOLLOW` or `KEPT`**, unless its `shows_count`
   is on, in which case a test may read its **total** (not its contents). So "draw the top two cards
   of the deck" is writable; "take the opponent's red card" and "if the deck holds an ace" are not.
   `every_action`, `must` and stages follow the same rule, and an end check may read anything (it
   ends the game, which reveals everything). Without `strict`, the checker does not enforce this, and
   a hidden bag's contents can leak through the options: the author's to keep clean, as the
   roll-peek leftovers are. The `HELD` rule (4) is not optional, since a choice shows its options
   outright. The game is told which bags are strict in `SPEC_FORMAT.md`, and the proof game uses it.
6. **Rolls and shuffles into hidden bags.** The dice-and-shuffles rule (no choice or bare test after
   a chance step) already stops a roll from being peeked at by the mover's options. A hidden bag
   adds the history (next point).
7. **The history.** `MoveRecord` is not sent today. A `Session::history_for(&Viewer)` returns it
   redacted so the first feature that shows a move list cannot leak: `rolled` faces and a held
   `chosen`/`piece_code` that came from a hidden bag are blanked to what the viewer may see (the
   die was rolled, the card was drawn, not which), and `shuffled` is already only a name.
8. **The state and the seed** stay in `game_instances.state`, server side, as now.
9. **Errors and revisions** are already viewer-independent. A test pins that a command which
   fails because of hidden contents gets the same answer as any other illegal command.

### "Try it" and the book

They use `Viewer::Everyone` (the wasm `Board.view()` is unchanged). They are one person playing every
side, so there is nothing to keep. A spec that hides a hand shows it in "Try it" and in the book; a
page that wants pass-and-play can ask for `Player(turn)` later.

## The slices

One pull request each, in order, each shippable alone. The first is this document.

**Slice 0, this design (docs only).** Frank rules on the list below; the document is amended.

**Slice A, the engine and the session.**
- `BagSpec.visible`, `shows_count` and `strict`, refused where meaningless; the checker's read-time
  rules (4 always, 5 for strict bags); a name for what a bag's visibility to the mover is, computed once.
- `Viewer`, `Session::view_for`, `Session::history_for`; the game reveal at game over; `material`
  withheld while it reads a hidden bag.
- Tests: each rule above, red first, and a pair of specs that read a hidden bag the same way, one
  strict (refused) and one not (accepted). Property test (the robustness harness): for every spec with
  hidden bags, random play, and at every step the serialised view of every viewer contains no code
  that the viewer may not see (a card that only a hand holds never appears in an opponent's view,
  and appears in the owner's).
- `SPEC_FORMAT.md`: a "Hidden information" section and the refusal rows.

**Slice B, the server and the web board.**
- The snapshot is built per viewer; the handlers' test that forbids `view()`; the HTTP tests: seat A
  never receives seat B's cards in a snapshot or a command response, a spectator and a caller with no
  token never receive any hand, after the game ends both can see everything.
- The web board shows a hidden bag as a count of card backs, the viewer's own hand face up, and a
  pile as a stack; the TypeScript types gain `hidden` and `count`.

**Slice C, the proof game.** A small original card-drafting game, "Draft": a shuffled hidden deck, a
row of face-up cards, each player's hand hidden with its size shown, a turn taking a card from the row
and refilling it from the deck, the highest set at the end (with the `BAGGED` score term, withheld
until the game is over). Played to the end over HTTP. Whether it joins the catalogue is Frank's call;
by default it lives in tests and in `crates/session/tests/fixtures`, like the dice race.

## Approaches considered

1. **Redact in the game server** (build the full view, then blank fields). One place, but a new
   field in the view is public until someone remembers to blank it, and the wasm/session view and
   the server's would differ. Refused: the anti-goal.
2. **The engine builds the view per viewer (chosen).** The decision lives next to the data; a new
   view field must choose what each viewer sees, because the function has no full-view default.
3. **Encrypt or hash hidden contents** so the browser could hold them and prove them later. A
   different problem (commit and reveal between mutually distrusting browsers); this platform has a
   trusted server, so the server keeping them is simpler and stronger. Refused.
4. **Let any test read a hidden bag and refuse nothing at read time**, leaning on review. The
   mover's legal options would tell them what they may not see. Kept as the default (`strict` is off),
   with the strict rule available per bag for a game that wants the engine to guarantee it, at the
   price of games that then cannot be written until a later block (peeking, searching a deck).

## Settled by Frank (2026-11-28)

1. **Reveal at game over** for every bag.
2. **`visible` has three values** (`ALL`, `OWNER`, `NONE`), `shows_count` is separate and on by
   default, and a shared bag with `OWNER` is refused.
3. **The strict read-time rule is optional**, per bag (`strict`, off by default), not always on. *Read
   as:* an author who wants the engine to guarantee that the rules do not read a hidden bag turns it
   on; one who does not, may write searching and asking mechanics and carries the leak. If the intent
   was the other way round (on by default, with an opt-out), only the default changes.
4. **A hidden bag that feeds `material` or `score` withholds the number** until the game is over; it is
   not blanked to zero.
5. **Map holder places stay public.** A hidden place on the board is not in this step.
6. **"Try it" shows everything** (hot seat).
7. **The proof game goes in tests**, not the catalogue.

## Left to the builder

The names of the fields, the viewer and the view's `hidden` and `count`; how the checker computes
"what the mover may see" (it is a function of the bag's declaration and the action's mover, and
`of`); the property test's size; the exact shape of the redacted history record; the card-backs'
drawing.

## As built, slice A (2026-11-28)

Built on `feature/hidden-information-engine`, as designed, with these notes and findings.

- **Names.** `visible` is `ALL`, `OWNER` or `NONE` (`Visible::{All, Owner, Nobody}` in Rust);
  `shows_count` and `strict` are optional booleans, so that writing one on a bag everyone sees is
  refused (a default would hide that the author wrote it). `HolderInfo` and `BagContents` carry
  `visible` and `shows_count`; a map place always has `ALL`.
- **The checker** is one module, `specs/game/hidden.rs`: the bags' own fields, then one walk over
  every place that runs steps or tests (actions, `every_action`, `must`, a stage's `only`, a piece's
  moves and repeats; not end checks). "What the mover may see" is a function of the bag's `visible`,
  the `of` and the action's `players`: `MOVER`'s own bag, always; a named player's, only in an action
  written for that player alone; the other player's, never. Rule 4 (`HELD`) is enforced in the same
  walk with or without `strict`.
- **`Viewer`** is in the session crate (`Player(name)`, `Spectator`, `Everyone`).
  `Session::view_for` builds the view; `view()` is `view_for(&Everyone)`. Only the mover's viewer has a
  `choice`; the other seats and a spectator get an empty one, so the game server's own emptying of it
  (slice B) becomes redundant.
- **Material** is withheld **per player**: a player's number is left out while any bag of theirs is
  hidden from the viewer, so a seat still sees its own total. No score term reads a bag yet, so the
  score rule has nothing to do until the `BAGGED` term of slice C.
- **History.** `MoveRecord` gained `private_bags`: the bags a turn took from, changed or rolled into
  that are not `ALL` (left out in a game that hides nothing, so its history and saved state are as
  they were). `history_for` blanks every held and chosen code and the `piece_code` of a turn that
  touched a bag the viewer may not see, and the face of a die rolled into one. It is conservative: a
  turn that also touched a public bag is blanked whole. Nothing is sent over HTTP yet.
- **Found in the review pass** and fixed: a non-mover was shown the mover's turn in the making (the
  board with the chosen card on it) through `shown_pieces`, so in a game that hides something the
  other viewers get the committed board; a turn with two `HELD` choices only blanked by the last bag
  (the property test found it before the review did); `CODE` answers feeding a hidden bag stayed in
  the history; and `REPEATS` compares every bag, so it is refused where a bag is `strict`. A `PLACE`
  choice over a strict bag is refused even when its count shows. Left as the author's, in
  `SPEC_FORMAT.md`: a bag with a `capacity` tells its fullness by whether `MOVE`, `PLACE` or `ROLL`
  into it is offered.
- **A finding for Frank.** Strict does not close one path: `MOVE` of `count` N from a hidden bag is
  offered only when the bag has N, so with `shows_count: false` a strict bag's size leaks as "at least
  N". It is written in `SPEC_FORMAT.md` as the author's to keep clean. Closing it would mean refusing a
  numeric `count` (other than `ALL`) on a strict bag that hides its count; left as a later call.
- **Verification.** 33 new engine tests (`tests/hidden`) and 24 session tests (`tests/hidden.rs`, with
  the fixture `secrets.json`, a card game of a hidden deck and hands), among them the property test
  (60 seeds of random play, every viewer, every step: no card they may not see is in their view or
  history; every card they may see is in their view), checked to fail when the hiding is turned off.
  The engine suite (826), every session test binary but `boards.rs` (needs the book's derived boards),
  `make lint-panics`, the workspace build and the wasm check pass.

## As built, slice B (2026-11-28)

Built on `feature/hidden-information-server-web`, stacked on slice A's branch.

- **The snapshot is built per viewer** (`crates/game/src/http/play.rs`): a seated caller gets
  `view_for(Player(seat))`; a signed-in stranger and a caller with no token get `view_for(Spectator)`.
  The handlers read the turn and the status from `onlooker()` (a spectator's view, which every
  viewer is told alike), so nothing in `src/` calls `.view()`: a test reads every source file of the
  crate (outside test modules) and fails if one does.
- **A match that has ended shows everything**, whatever ended it. A resignation does not move the
  engine's phase to game over, so the snapshot of a match with an outcome is built for `Everyone`
  with the choice blanked, which reveals the hands after a resignation as it does after a win.
  (A finding: the engine's reveal follows its phase, and a resignation is the server's.)
- **The choice** is no longer emptied by the server for non-movers (the engine does it); the server
  only blanks it for a finished match.
- **The web board**: `BagView` gains `hidden` and `count`; the play island draws a hidden bag as up
  to five card backs and `×count` (or "Hidden" with no count), with an accessible label, never as
  buttons; a hidden bag known to be empty is not drawn. A withheld `material` already left the
  captured panel without a lead. "Try it" and the book are unchanged.
- **Verification.** Six new HTTP tests (`tests/play.rs`: a seat, a stranger and an anonymous caller
  never receive the other seat's card or the deck's, in snapshots or command responses; only the
  mover has a choice and it names only their own cards; a guess at the other hand is refused like a
  card that is nowhere; a finished and a resigned match show everything; no handler uses `view()`),
  six web tests. The game crate (44 `play` tests; `catalog.rs`'s known failure aside), 210 web
  tests, `tsc`, `make lint-panics`. The web tests need Node 22.12 or newer (Homebrew's `node` works).

