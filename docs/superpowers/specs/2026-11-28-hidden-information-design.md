# Hidden information (step 7)

Step 7 of `2026-11-21-engine-generalisation-design.md`, after dice and shuffles
(`2026-11-27-dice-and-shuffles-design.md`, merged as #89). A design only: this document and its
slices are the whole of the first pull request. Every call below is **Claude's**, to be ruled on by
Frank; the ones most worth his eye are listed at the end under "For Frank".

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

Two optional fields on a bag:

```json
{ "name": "HAND", "for": "EACH_PLAYER", "visible": "OWNER" },
{ "name": "DECK", "ordered": true, "visible": "NONE", "shows_count": true },
{ "name": "ROW" }
```

- `visible`: who sees the **contents**. `"ALL"` (the default: everyone, as every bag today),
  `"OWNER"` (the player a bag belongs to; for a shared bag, which belongs to nobody, it is the same
  as `NONE`, and the checker says so), or `"NONE"` (nobody until the game is over).
- `shows_count`: whether everyone sees **how many** pieces it keeps when they cannot see which.
  Default `true` (a hand's size and a deck's height are public in nearly every game). Meaningless,
  and refused, with `visible: "ALL"`.

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
   leaks it. The checker enforces one rule, conservatively: **in an action, a bag the mover may not
   see can only be named as the source or target of `MOVE`, `REMOVE`, `PLACE`, `ROLL` or
   `SHUFFLE`, with no `what`, and is never read by a test, a `COUNT`, a `count` region, `FOLLOW` or
   `KEPT`**, unless its `shows_count` is on, in which case a test may read its **total** (not its
   contents). So "draw the top two cards of the deck" is writable; "take the opponent's red card" and
   "if the deck holds an ace" are not. `every_action`, `must` and stages follow the same rule, and an
   end check may read anything (it ends the game, which reveals everything).
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
- `BagSpec.visible` and `shows_count`, refused where meaningless; the checker's read-time rules (4
  and 5); a name for what a bag's visibility to the mover is, computed once.
- `Viewer`, `Session::view_for`, `Session::history_for`; the game reveal at game over; `material`
  withheld while it reads a hidden bag.
- Tests: each rule above, red first. Property test (the robustness harness): for every spec with
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
   mover's legal options would tell them what they may not see. Refused for rule 5 above, at the price
   of some games that cannot be written until a later block (peeking, searching a deck).

## For Frank

1. **Reveal at game over** for every bag. (The alternative: only what the rules reveal, with a
   `reveal` effect, a later block.) Chosen because every card game this targets does it.
2. **`visible` has three values** (`ALL`, `OWNER`, `NONE`) and `shows_count` is separate and on by
   default. Ruling wanted on the names, and on whether a shared bag with `OWNER` should be refused
   instead of read as `NONE`.
3. **The read-time rule 5 is strict**: some games (searching a deck, an opponent's hand by name) are
   unwritable until a later block adds peeking or a `reveal`. A looser rule would let the spec author
   leak by accident.
4. **A hidden bag may not feed `material` or `score` while the game runs**: withheld, not blanked to
   zero. A viewer would otherwise see a number that moves for a reason they cannot see.
5. **Map holder places stay public**, so a game cannot hide a pile on the map. A hidden *place* on
   the board (fog of war) is not here at all.
6. **"Try it" shows everything** (hot seat). A pass-and-play mode that shows the mover's hand and
   hides the others is a later page feature, not a spec one.
7. **The proof game goes in tests**, not the catalogue, as the dice race did; its page, image and seed
   entry are their own work.

## Left to the builder

The names of the fields, the viewer and the view's `hidden` and `count`; how the checker computes
"what the mover may see" (it is a function of the bag's declaration and the action's mover, and
`of`); the property test's size; the exact shape of the redacted history record; the card-backs'
drawing.
