# Actions and effects (slice 2 of the generality pass)

**Status:** approved by Frank on 2026-11-22, with his answers in "Frank's answers" at the end.
Decisions marked *Claude's call* stand unless he corrects them.

## Why

Slice 2 of `2026-11-22-rule-blocks-design.md`. Frank, 2026-11-22: a move is a general thing, and
a single piece's move is one simple case. Today the engine knows four kinds of turn: a piece
move, a drop, a pass and a promotion. Each has its own phase, its own transition, its own
session method, its own game-service command and its own corner of the board element. What a
move *does* is spelled with special forms keyed by what stands on the target (`EMPTY → MOVE`,
`ENEMY → CAPTURE`), a fixed list of side effects (`CAPTURE` at an offset, `MOVE`, `CONVERT`,
`COPY_SOURCE`, `EXPLODE`, `SET_STATE`), and `modifiers` for promotion. A game whose turn is not
one of the four (place a piece and then remove one, move and then shoot, choose a number in 4c,
play a card in step 7) cannot be written.

After this slice there is one kind of turn. An **action** is what a player can do on their
turn: the choices they make, the tests that allow it, and the effects it has. A piece move, a
drop, a pass and a promotion are ordinary actions, written with the same blocks.

## What this slice covers

From the route: it replaces the occupancy-keyed `actions` (`MOVE`/`CAPTURE`), every side effect
(`CONVERT`, `COPY_SOURCE`, `EXPLODE`, `CAPTURE` at an offset, `MOVE`, `SET_STATE`), `modifiers`,
`allow_pass`, and the separate drop and promotion phases. The session, the wasm wrapper, the
game service, the web app and the board element move to one protocol.

Also removed here (*Claude's call*): `unlimited_supply` (a `PLACE` effect creates the stone, so
Go and Hex no longer need a supply) and `drop_conditions` (they become the drop action's own
tests). The route had left `unlimited_supply` to 4c; it falls out of this slice for free.

Not in this slice, as the route says: hands as places and `hand_enabled`/`demotes_to` (4c),
`jump_enabled` (step 5), `mandatory_capture`, `leader`, `no_opponent_check` and the attack
sets' handling of tests (slice 3), counters and ends (slice 4). `threat_only` stays until
slice 3, which owns attack sets.

## Goals, non-goals, anti-goals

Goals, beyond the pass's own (every catalogue game plays as today; grid speed within 5%):

- One turn vocabulary: every spec's turns are actions made of choices, tests and effects.
- One client protocol: the view says what is being chosen and what answers it; every click or
  button is an answer.
- Piece moves keep today's feel: click a piece, see where it can go, click there; a promotion
  is asked after the move.

Non-goals: new games (beyond test specs that prove a block), turn stages, the builder.

Anti-goals:

- An effect or choice that only one game can use.
- A spec that silently changes meaning: unknown fields are refused on every new block (the
  carried item "side effects do not refuse unknown fields" is closed here).
- A slower hot path: piece-move actions compile to today's generator.

## Approaches

Three ways were considered.

1. **Steps in order** (Claude recommends). An action and a piece's move are each a list of
   steps: choose, test, effect and `if`. They run in order on the position as it changes, and
   a test judges the position as the steps before it left it. A promotion is just "after the
   move, if in the zone, choose a code, then set it". Choices can come after effects, which
   step 5 (turn stages) and 4c ("how many") build on directly. The cost is that enumerating a
   turn means walking the steps and branching at each choice. A move's tests that come before
   its first effect still compile to today's fast filter.
2. **Fixed parts.** An action is choices, then tests, then effects, in that order. A promotion
   needs a choice with a `when` test plus an effect that repeats the same test (or two copies
   of every pawn move, with and without promotion; Shogi would have three of each). This is
   simpler to enumerate, but the duplicated tests are easy to get out of step, and a choice
   after an effect has no place. Step 5 would reopen the shape.
3. **Rename only.** Keep piece moves as they are, and add drops, passes and promotions as
   actions beside them. This was dropped because it keeps every special form the route lists.

The rest of this document assumes approach 1.

## The blocks

### Actions

The spec's top-level `actions` lists what a player may do on their turn:

```json
"actions": [
  { "name": "MOVE", "do": [
      { "choose": "PIECE", "where": { "owner": "ALLY" } },
      { "choose": "MOVE" }
  ] }
]
```

| Field | Meaning |
|---|---|
| `name` | Unique within the spec. It is shown in the history and labels buttons. |
| `players` | Optional. The players who may take it. Empty or absent means everyone. |
| `do` | The steps, in order. An empty list is a pass. |

The engine knows no action by name. "MOVE", "DROP" and "PASS" are only the names the catalogue
uses.

### Steps

A step is exactly one of four things, told apart by its first key:

| Step | Written | Does |
|---|---|---|
| choose | `{ "choose": KIND, ... }` | Asks the player (see Choices). |
| test | `{ "test": ... }` | A slice-1 test, judged on the position as the earlier steps left it. If it fails, this way of taking the action does not exist. |
| effect | `{ "effect": KIND, ... }` | Changes the position (see Effects). |
| if | `{ "if": TEST, "then": [steps], "else": [steps] }` | Runs `then` when the test holds, otherwise `else`. Either list may be empty or absent. |

`START` and `TARGET` are set by choices (below). A place a step needs but that no earlier
choice set is refused when the spec is read.

### Choices

| Choice | Written | Options | Sets |
|---|---|---|---|
| `PIECE` | `{ "choose": "PIECE", "where": F }` | the places holding a piece that matches filter `F` | `START` |
| `MOVE` | `{ "choose": "MOVE" }` | the places the piece at `START` reaches by its moves (below) | `TARGET`, then runs that move's own steps |
| `PLACE` | `{ "choose": "PLACE", "in": R }` | the places of region `R` | `TARGET` |
| `HAND` | `{ "choose": "HAND", "where": F }` | the codes in the mover's hand that match `F` (`codes` only) | `START`, as "this piece in the hand" |
| `CODE` | `{ "choose": "CODE", "as": NAME, "options": [codes] }` | the listed codes | the value `NAME` |

Some rules about choices:

- **Only options that can finish are offered.** An option is offered only if some complete way
  of taking the action through it passes every test after it, including the rules that apply
  to every move (leader safety, liberties and repetition, as today). The player is never shown
  a dead end.
- **`MOVE` needs a piece on the board.** It is refused when the spec is read unless an earlier
  `PIECE` sets `START`.
- **`HAND` is a bridge until 4c.** In 4c the hand becomes a real place, and `HAND` becomes a
  `PIECE` choice there.
  - In this slice, `START` names "the chosen piece in the mover's hand".
  - `COUNT` in `AT START` sees that piece, so `codes` tests work, which is how Shogi's pawn-only
    drop rules are written.
  - An offset from `START` cannot be formed (false, as in slice 1).
  - This lifts slice 1's "START is refused in a drop", since a drop now has a moving piece.
- **A new region, `BOARD`:** `{ "region": "BOARD" }` is every place of the board. It is used to
  choose any place for a drop or a stone. It works on grids and maps alike.
- **Every choice takes an optional `as` name** (*Claude's call*). Only `CODE` needs one in this
  slice. Naming a second place, say for Amazons' arrow, is left to step 5.

### A piece's moves

`pieces[].moves[]` keeps listing where a piece can go. `step`, `repeat` (with slice 1's
`until`), `players` and `threat_only` stay as they are. Everything else becomes `do`:

```json
{ "step": [0, 1], "repeat": { "loop": true }, "do": [
    { "test": "NOT", "of": { "test": "COUNT",
        "in": { "region": "AT", "at": { "place": "TARGET" } }, "where": { "owner": "ALLY" } } },
    { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } }
] }
```

`step` and `repeat` list the candidate `TARGET`s, exactly as today. For each candidate, the
move's `do` runs with `START` the piece's square and `TARGET` the candidate. The tests before
its first effect are today's conditions and stay on the fast path.

The occupancy key goes away. Today's `EMPTY → MOVE` plus `ENEMY → CAPTURE` on one move becomes
"not an ally at `TARGET`", then `MOVE`. A move that did different things on an empty and an
enemy square becomes two moves with the same step. One example is the pawn's diagonal:
a capture, and en passant.

**Two moves reaching the same `TARGET`.** When two moves of one piece both reach the same
`TARGET`, and give the same answers, the first in spec order is kept. Today one silently
overwrites the other in a map. (*Claude's call*. No catalogue game does this on purpose.)

### Effects

| Effect | Written | Does |
|---|---|---|
| `MOVE` | `{ "effect": "MOVE", "from": P, "to": P }` | The piece at `from` goes to `to`; whatever stood on `to` is removed (as `REMOVE`). From a `HAND` `START`: one piece of that code leaves the hand and enters `to` as a new, unmoved piece (as a drop does today). |
| `REMOVE` | `{ "effect": "REMOVE", "at": P }` | The piece at `P`, if any, leaves the board. Where hands are kept, an enemy piece removed by the mover goes to the mover's hand, demoted as today. |
| `PLACE` | `{ "effect": "PLACE", "at": P, "code": C }` | A new, unmoved piece of code `C`, owned by the mover, is put on `P`. Whatever stood there is removed first. |
| `SET_OWNER` | `{ "effect": "SET_OWNER", "at": P }` | The piece at `P`, if any, now belongs to the mover. |
| `SET_CODE` | `{ "effect": "SET_CODE", "at": P, "code": C }` | The piece at `P`, if any, becomes code `C`. It keeps its owner, and is otherwise new (as a promotion is today). |
| `SET_FLAG` | `{ "effect": "SET_FLAG", "at": P, "flag": "EN_PASSANT", "turns": 1 }` | Sets a flag on the piece at `P`. With `turns`, it expires as today's timed state does; without it, it lasts. |
| `FOR_EACH` | `{ "effect": "FOR_EACH", "in": R, "do": [steps] }` | Runs `do` once for each place of `R` (taken before any of them runs), with the place `EACH` set to it. `do` may hold effects, tests-in-`if` and `if`, but no choices. |
| `END_TURN` | `{ "effect": "END_TURN" }` | The turn ends when this way is played, even where a continuation would follow (checkers' crowning; see Frank's answer 5). |

Values that effects take:

- **A place `P`:** slice 1's place block, plus `{ "place": "EACH" }` inside `FOR_EACH`.
  Offsets are turned per player, as everywhere.
- **A code `C`:** a piece code written as a string, or `{ "chosen": NAME }` for a `CODE`
  choice's answer. An unknown code, or a name no earlier choice set, is refused when the spec
  is read.
- **The owner:** always the mover in this slice. An owner field arrives when a game needs one.

**Has moved.** When an action is played, the piece its `PIECE` choice picked counts one more
move, wherever it ends up. No effect counts moves. This keeps today's counts exactly: a castled
rook is moved by an effect and stays unmoved, as now, which matters because the repetition key
includes each piece's has-moved flag. A piece dropped from the hand enters unmoved.
(*Claude's call*, after the builder check.)

`MOVE` replacing what stands on `to` is *Claude's call*, and question 2 for Frank. "Add to a
counter" from the route's sketch is left to slice 4, which introduces counters.

### What a turn is, and what is a capture

- **A way of taking an action** is the list of answers given and the changes it makes. The
  answers are pieces, places, codes and a hand piece. The changes are recorded as a list, never
  as a copied position. Slice 5 makes that list lean.
- **A capture** is a way that removes an enemy piece, by `REMOVE`, `MOVE` onto it or `PLACE`
  onto it. Three rules that stay until their slices use this definition in place of today's
  `"CAPTURE"` action string:
  - `mandatory_capture` and `jump_enabled`'s continuation (checkers);
  - the fifty-move rule's reset;
  - the history's capture mark.
- **A pass** is a way that changes nothing. Go's "both passed" counts these.
- **Attack sets.** The attack set (slice 3 reworks it) stays as today. A move contributes
  threats unless a test before its first effect requires `TARGET` to be empty. This reproduces
  today's "only moves with an `ENEMY` action threaten" for every catalogue spec.
- **Atomic's kings.** Atomic's "a capture that would blow up the capturer's own king is no
  threat" no longer looks for `EXPLODE`. When the spec is read, the engine notes whether any
  move can remove more than its target: a `REMOVE` inside `FOR_EACH`, or a `REMOVE` at
  `START`. Only then does check detection simulate a capture of a leader, as it does today
  only when there are explosions. Other games never pay for the simulation, which clones the
  game. (*Claude's call*, after the builder check found that an ungated simulation would land
  on chess's legality filter.)
- **Behaviour changes, named:**
  - A capture that promotes now carries the capture mark in the history. Today it is recorded
    as `TRANSFORM` and has none.
  - A promotion with one option (checkers' king) no longer asks the player. The piece simply
    becomes that code.
  - Checkers' crowning jump: see question 5.

### How the catalogue reads after the rewrite

Pawn: the forward step, with promotion. A one-option promotion (checkers' king) is a plain
`SET_CODE` with no choice.

```json
{ "step": [0, 1], "do": [
  { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } } } },
  { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } },
  { "if": { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "REACH_END" },
    "then": [
      { "choose": "CODE", "as": "promotion", "options": ["QUEEN", "ROOK", "BISHOP", "KNIGHT"] },
      { "effect": "SET_CODE", "at": { "place": "TARGET" }, "code": { "chosen": "promotion" } } ] }
] }
```

| Today | As steps |
|---|---|
| Double step + `SET_STATE EN_PASSANT 1` | tests; `MOVE`; `SET_FLAG` at `TARGET`, `EN_PASSANT`, `turns: 1` |
| En passant (`EMPTY → MOVE` + side effect `CAPTURE [1,0]`) | its own move with the same step: target empty; `COUNT` at `START+[1,0]` with flag; `REMOVE START+[1,0]`; `MOVE` |
| Castling (side effect `MOVE` of the rook) | tests; `MOVE START→TARGET`; `MOVE START+[3,0] → START+[1,0]` |
| Checkers jump (`EMPTY → CAPTURE` + `CAPTURE [1,1]`) | target empty; enemy at `START+[1,1]`; `REMOVE START+[1,1]`; `MOVE`; `if` end zone, `SET_CODE KING` and `END_TURN` |
| Ataxx one-step (`COPY_SOURCE` + 7 × `CONVERT`) | target empty; `PLACE` at `TARGET` `STONE`; `FOR_EACH` in `AROUND TARGET`: `if` enemy at `EACH`, `SET_OWNER EACH` |
| Ataxx two-step (8 × `CONVERT`) | target empty; `MOVE`; the same `FOR_EACH` |
| Atomic capture (`ENEMY → CAPTURE` + `EXPLODE` sparing pawns) | enemy at target; `REMOVE TARGET`; `REMOVE START`; `FOR_EACH` in `AROUND TARGET`: `if` not a pawn at `EACH`, `REMOVE EACH`. A pawn's capture has no promotion steps, because the pawn is gone (today nothing is asked either) |
| Shogi promotion (forced zone, optional zone) | `MOVE`; `if` forced zone: `SET_CODE TOKIN`, `else if` promotion zone: choose `TOKIN`/`PAWN`, `SET_CODE` |
| Go stone (`unlimited_supply`, drop) | action `PLACE`: choose `PLACE` in `BOARD`; target empty; `PLACE` `STONE` at `TARGET` |
| `allow_pass` | action `{ "name": "PASS", "do": [] }` |
| Shogi drop (hand + `drop_conditions`) | action `DROP`: choose `HAND`; choose `PLACE` in `BOARD`; target empty; `ANY [ not a pawn at START, ALL [the nifu and last-rank tests] ]` (likewise lance and knight); `MOVE START→TARGET` |

## The protocol

One protocol from the engine to the board.

**The engine.**
- It keeps the answers given so far this turn, and works out every complete way of taking each
  action from the position. This is what the speed run already does for piece moves.
- `GamePhase::{Idle, Moving, Transforming, Dropping}` become one `Choosing` phase. `GameOver`
  stays.
- The transitions become:
  - `Choose(Answer)`, where an `Answer` is a place, a code, a hand piece or an action name;
  - `Cancel`, which clears the answers.
- Once the answers match exactly one complete way, it is played.
- Checkers' jump continuation stays until step 5. The next turn starts with the piece already
  chosen, only captures are offered, and `Cancel` is refused.

**The view.** `selected`, `destinations`, `promotion`, `drops` and `can_pass` are replaced by
one `choice` object:

```json
"choice": {
  "chosen": [{ "place": "4,1" }],
  "start": "4,1",
  "places": [{ "name": "4,2", "for": "MOVE" }, { "name": "4,3", "for": "MOVE" }],
  "codes": [],
  "hand": [],
  "actions": []
}
```

The fields mean:

- `chosen`: the answers given so far.
- `start`: the chosen piece's square, drawn as selected.
- `places`: each place that answers the next choice, with the kind of choice it answers. The
  board element marks `MOVE` and `PLACE` places as destinations, and leaves `PIECE` places
  clickable without a mark, as today.
- `codes`: a promotion, or any code choice.
- `hand`: the hand pieces that can be chosen.
- `actions`: actions that need no choice right now. Each one is a button, like today's pass.

At the start of a chess turn, `places` lists the pieces that can move. In Go it lists the empty
points, plus `actions: ["PASS"]`. In Shogi it also has the droppable hand pieces in `hand`. The
game service hides `choice` from everyone but the player to move. That also fixes today's leak
of `promotion` to the other player.

**Clicks keep today's feel** (*Claude's call*, in the session, not the engine). Clicking one of
your own pieces while a `MOVE` choice is open cancels and chooses that piece instead, as
clicking another piece does today. Clicking a place that answers nothing cancels.

**Commands.**
- The game service's `select`, `move`, `promote`, `drop` and `pass` become:
  - `{ "type": "choose", "place": "4,2" }`, or the same with `"code": "QUEEN"`,
    `"hand": "PAWN"` or `"action": "PASS"` (exactly one of these keys);
  - `{ "type": "cancel" }`;
  - `resign` stays.
- The 64-byte limit applies to every string in them.
- The session and the wasm wrapper get matching `choose(...)` and `cancel()` methods.
- A drop becomes two answers, hand piece then square. Today it is one atomic command after a
  pick that is local to the page. So choosing a hand piece in a live match is now a round
  trip, like selecting a piece already is (question 3 for Frank).

**The history.** Each record becomes `{ player, action, chosen, piece_code, from, to, captures }`:
- `action` is the action's name, and `chosen` is the answers;
- `piece_code`, `from` and `to` are filled from the answers where they exist, for the move
  list and the fifty-move rule;
- `captures` marks a capture;
- `promotion` is replaced by the code in `chosen`.

**Saved games.**
- As in step 4a, matches saved before this branch are not kept. Nothing is in production
  (question 4 for Frank).
- A migration rewrites logged commands where one maps onto one, as `0004` did:
  - select and move become a place;
  - promote becomes a code;
  - pass becomes the action `PASS`;
  - a logged drop becomes one row carrying both answers, which the log type reads as two.
- How old rows and old states are read is left to the builder, within that rule.

**While a later choice is open** (a promotion is the common case), the view's `pieces` show
the position the effects so far have made. The pawn stands on the last rank, as it does today,
and the opponent sees it there too. `Cancel` is refused once an effect is showing, so a
promotion cannot be backed out of, as today. Before any effect runs, `Cancel` works as
deselecting does today. (*Claude's call*, after the builder check.)

**Telling ways apart by action.** An action's name is always a valid answer. It narrows the
ways to that action's, and it can be given at any point. When the answers so far match more
than one complete way, and the ways come from different actions, the view lists those actions
in `actions`, and the player picks one. Two cases:
- Two placing actions, say `STONE` and `DISC`, both choosing a place. The player picks the
  piece first (as the supply chips do today) or after clicking the point. The web test "with
  two supply pieces one has to be chosen first" keeps its meaning.
- A complete way plus longer ways that share its answers. The shorter way's action is offered
  beside the next choice.

Ways from one action that give the same answers keep the first in spec order.

In `actions`, each entry says whether answering it plays the action at once (a pass) or only
narrows the ways (the old supply pick): `{ "name": "PASS", "plays": true }`. `view.supply` goes.
The board element's click-to-place and the play island's "Place a stone…" hint read the
narrowing actions instead. (*Claude's call*.)

## Speed

- A piece-move action (`PIECE`, then `MOVE`) compiles to today's generator. The tests before a
  move's first effect stay as its fast filter.
- Effects compile into the same kind of change list as today's board changes, so nothing is
  copied that is not copied today. Slice 5 then makes the change list lean.
- The view needs every complete way at the start of a turn, to list the movable pieces. That is
  the full legal-move set, which is the speed run's own workload, once per turn instead of once
  per click.
- Promotions now count one way per option, and Shogi's third ply has optional promotions.
  So the speed run and the catalogue count tests both count distinct piece-and-target pairs,
  not ways, which keeps their numbers identical to `main`'s. (*Claude's call*.)

## Checked when a spec is read

Each of these is refused with a readable error:

- a step with none, or more than one, of `choose`/`test`/`effect`/`if`;
- an unknown choice, effect or field (every new block is strict);
- `MOVE` without an earlier `PIECE`;
- `START`, `TARGET` or `EACH` used where nothing sets it;
- a `{ "chosen": NAME }` that no earlier `CODE` choice names;
- an unknown piece code;
- a choice inside `FOR_EACH`;
- duplicate action names;
- a spec with no actions.

The old forms are refused with an `OldForm` error naming the replacement:

- a move's `actions`, `conditions`, `modifiers` and `side_effects`;
- `allow_pass`, `unlimited_supply` and `drop_conditions`.

## Proving it

- **The catalogue is rewritten.** Every catalogue spec, every book board, every spec inside a
  test and the CLI are rewritten. Every existing test keeps what it checks.
  - The catalogue count tests from slice 1 are the equivalence proof: identical counts at
    their depths for every game.
- **New engine tests:**
  - each choice, effect and `if`, `FOR_EACH`, the `BOARD` region and each refusal;
  - a turn made only of choices after effects;
  - a test spec that places a piece and then removes an enemy one, a kind of turn today's
    engine cannot express.
- **Session and game service.** Session tests and the robustness driver use the new protocol,
  and so do the game service's HTTP tests and the web tests. The jump lock, the promotion
  hidden from the other player, and a drop as two answers each get a test.
- **Speed.** The timing run stays within 5% of `main` on chess, Shogi and Go, with identical
  counts.
- **Docs.**
  - `specs/SPEC_FORMAT.md` documents actions, steps, choices and effects.
  - The book teaches them in the chapters on moves, promotion, conditions, the hand, placing
    stones and designing a game. The passages that say "select" are reworded.
- **In a browser:** chess (castling, en passant, promotion), Shogi (a drop, an optional
  promotion), Go (a stone, a pass), checkers (a double jump), Ataxx and Atomic each play in the
  book, and one live match through the web app.

## Frank's answers (2026-11-22)

The draft asked five questions; Frank answered:

1. **Steps in order.** The design above uses them.
2. **`MOVE` onto an occupied place removes what is there**, as long as the change is still
   noted. A capture must still show what material was taken.
   - So every removal, by `REMOVE`, `MOVE` or `PLACE` onto a piece, appears in the change list
     as a removal of that piece.
   - The captured-material count, the hand and the history's capture mark are all worked out
     from those removals, never from the action's name.
3. **Picking a hand piece as an answer sent to the server** is completely acceptable.
4. **Matches saved before this branch are not kept.**
5. **Checkers follows the standard rules, and they must be written in blocks**, not as a quirk
   of the engine. In English draughts that means:
   - a crowning jump is a capture;
   - it is forced like any other jump;
   - it may end a multi-jump;
   - crowning ends the turn.

   The first three follow from the capture rule above. The last is written in the spec: a man's
   jump that crowns it says so with a step that ends the turn (`{ "effect": "END_TURN" }`).
   That stops the jump continuation, which otherwise offers the same piece's next capture.
   `END_TURN` is a general effect: any move may end the turn early, even where a continuation
   would follow. Step 5 keeps it when turn stages replace the continuation. (*Claude's call*
   on the spelling.)

## Left to the builder

- Type names, file layout, how steps compile and how ways are enumerated.
- Error texts, in today's style.
- The board element's exact marks and button styles for `actions` and `codes`, keeping today's
  look where it exists.
- The wasm method names.
- How old command rows and old states are read, within the rule above.
- Commit order, and how the book changes across commits.
