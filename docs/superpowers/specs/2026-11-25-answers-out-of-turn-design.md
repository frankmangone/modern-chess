# Answers out of turn (step 8b)

Step 8 of `2026-11-21-engine-generalisation-design.md`, second half; the first half, sides with
their own rules, is built (`2026-11-25-sides-with-own-rules-design.md`). Written after a brainstorm
with Frank: his rulings are marked **Frank**, everything else is **Claude's** call and Frank may
overrule it.

## Intent

A rule can ask other players for a decision in the middle of a turn, and the turn waits for the
answer: everyone discards down to seven, a trade offer, a draw offer, the two players of a Go game
agreeing which stones are dead, both players choosing at once in rock-paper-scissors. Not every
game asks the same way, so the ask has a **mode**.

## Settled by Frank

1. **Different games ask differently, so there are modes:** one after another, and all at the same
   time (answers sealed or in the open). The design covers all of them; they are **built in two
   slices**, each its own pull request.
2. **The rule says what follows the answers:** the asker moves again in a stage the rule names, or
   the turn is handed on.
3. **An absent seat is not handled here.** A seat that never answers blocks the match as an absent
   mover does today; resigning is the way out, and clocks (a platform feature) will cover moves and
   answers together. Documented as a known limitation.
4. **The proofs are Go's dead-stone agreement** (one after another) **and rock-paper-scissors**
   (all at the same time, sealed, and the same game with the seal off for the open mode).

## Goals, non-goals, anti-goals

Goals: one new effect, `ASK`, that hands a decision to other players and the turn back or on;
asked seats answer with the same ways, choices, answers and tests as anyone; the in-order mode
keeps the engine's one-mover model; the all-at-once mode lets seats answer in any order and,
sealed, keeps each answer from the others until every seat has answered; hidden information
(step 7) holds throughout; a stored game and every existing spec are unchanged.

Non-goals: timeouts and forfeits (settled above); a general "interrupt" that answers a move
already played (a reaction card that cancels the last effect: it would need the effect undone, and
no game here needs it); a way to ask a different question of each seat in one ask (ask twice);
`ROLL` and `SHUFFLE` inside the answers of an all-at-once ask (their draws would depend on the order
the seats answered in; refused when the spec is read, until a game needs it).

Anti-goals: any change to how a game without an `ASK` plays, loads or saves; a second turn model
beside the first (an asked seat plays through the same ways, answers and `Game::choose` any seat
plays through); an answer that can be read by a seat that should not.

## The effect

```json
{ "effect": "ASK", "players": "OTHERS", "stage": "REPLY",
  "mode": "IN_ORDER", "then": { "stage": "AFTER" } }
```

| Field | Meaning |
|-------|---------|
| `players` | `OTHERS` (every player but the asker, in turn order after them), `ALL` (the asker last), `MOVER`'s `ENEMY` (two players), or a list of names. |
| `stage` | The stage the asked seats answer in, from `turns.stages`. |
| `mode` | `IN_ORDER` (default) or `TOGETHER`. |
| `sealed` | With `TOGETHER` only: true (default) keeps answers from the others until all are in; false lets each answer take effect as it arrives. |
| `then` | `{ "stage": NAME }`: the asker moves again in that stage. Absent: the turn is handed on, as after any way. |

The asking way is played in full first (its own steps are done), exactly as `STAGE` does today; the
ask takes the place of `STAGE`, and a way may not run both (refused when read).

**What an asked seat can do.** Its ways are the ways of the actions that **belong to the stage** it
is asked in. An action gains an optional `"stage": NAME`: it is offered only in that stage. An
action without it is offered where it is now (on a normal turn, and in a stage, as every stage
today) — so no existing game changes. A stage may also be given `players`, so a reply stage is not
entered by anyone it was not asked of. An asked seat answers exactly **one way** in the ask; the
way may `ASK` again, which is how "discard until you hold seven" is written (a way that is only
there while the hand is over the limit, ending in an `ASK` of the same seat).

**Who is skipped.** An asked seat with no way in the stage is skipped (as `STAGE` hands on when a
stage has no way). All seats skipped: the ask is over.

**From the asked seat's side** it is a turn: `MOVER` and `ALLY`/`ENEMY` in its tests are itself, and
`START`, `TARGET`, the history record and the hidden-information views are the asked seat's, so
none of what step 7 built needs to be taught about asks.

## Modes

### `IN_ORDER` — slice 1

Asked seats answer one after another. The engine's one-mover model is kept whole: the state gains a
queue of the seats still to answer (`asked`), and **`current_player()` is the head of the queue**
while it is not empty. Everything that reads the mover then reads the asked seat: the ways worked
out, the session's `turn`, the view's `choice` (only for that seat), the server's "whose turn is
it" check, the hidden-information viewer. When the queue empties the asker moves again in the `then`
stage, or the turn is handed on, and the end checks are judged as for any turn that ends.

End checks are judged when a turn is handed on (as now, and not between a stage's ways): an ask is
part of the asker's turn.

Nothing changes in the session, the server or the web board for this slice: the turn is simply
somebody else's for a while. (The board's "waiting for X" line already follows `turn`.)

### `TOGETHER` — slice 2

Every asked seat owes an answer at once, in any order. The one-mover model has to give: the state
holds the seats still to answer, each with its own part-way answers, and the engine's `choose` takes
the seat (`choose_as`; a single-mover game keeps `choose`). The ways of every asked seat are worked
out **at the moment of asking, on the same position**, so no seat's choices depend on how quickly
the others answer.

- **Open** (`sealed: false`): each answer is played as it arrives; the position changes for the
  seats yet to answer, whose ways are worked out again.
- **Sealed** (the default): an answer is kept, not played, until all seats have answered; then the
  answers are played together in turn order, as if one way. Seats are told only *that* a seat has
  answered (a lock, not what it chose). The history records the answers when they are played, so a
  sealed answer reaches no history before the reveal. Because they are played in turn order, answers
  that touch the same place or bag act in that order: the author keeps them independent (RPS puts each
  pick in its own bag), and this is written in `SPEC_FORMAT.md`.

The view gains `waiting`: the seats that have not answered. `choice` is the viewer's own while they
owe an answer, and empty otherwise (`is_mover` becomes "owes an answer or is the one to move"). The
server's rule that only the mover may command becomes "only a seat that owes an answer, or is the
mover". A race (two seats answering in the same moment) is the revision protocol as it is: one
command wins, the other gets `409` and refetches; the web board retries once on its own.

## Hidden information

An asked seat's `choice` is its own, and the step 7 checker's rules read the asked seat as the
mover (it may only name a bag it can see). In `IN_ORDER` nothing else is needed. In sealed
`TOGETHER` the kept answers are in the saved state, which never leaves the server, and the view
carries only `waiting`. A finished match is revealed as ever.

## The proofs

**Go's dead-stone agreement** (one after another). After two passes the game does not end: a
marking stage lets the player to move flag groups as dead (`SET_FLAG`, repeated by `STAGE`) and
finish; then `ASK` the other player to `AGREE` (the flagged stones are removed and the score is
taken) or `DISPUTE` (the flags are cleared and play resumes). `go.json` in the catalogue keeps
today's rule; the agreement is a variant in the tests (`known-limitations.md`'s "no dead-piece
agreement" is then a documented, written-down way to do it rather than a missing block). It proves
`IN_ORDER`, `then` (the asker resumes), actions that belong to a stage, and an ask answered more
than once.

**Rock-paper-scissors** (`TOGETHER`, sealed). Each round both players choose a hand and the
winner is read from the two. It proves sealed answers, the lock in the view, the reveal, the
history, the server over HTTP with both seats answering in either order, and — with `sealed: false`
— the open mode, where the second seat sees the first's pick. A drawn round is played again.

## Approaches considered

1. **A set of movers in the engine for every mode.** One mechanism, but the whole of slice 1 is then
   as large as slice 2. Refused: `IN_ORDER` covers most games and can ship alone.
2. **A turn that skips ahead to the asked seat** (change `current_turn`). Tempting, but `current_turn`
   is what the order and the repetition record are built on; a queue read by `current_player()`
   leaves them alone. Chosen.
3. **Out-of-turn commands outside the ways** (a server message "discard card X" that bypasses the
   rule engine). Refused: every answer is a way, checked by the rules.
4. **A new kind of action for replies.** The `stage` field on an action reuses what actions are.

## Left to the builder

The names (`ASK`, `OTHERS`, `IN_ORDER`, `TOGETHER`, `sealed`, `then`, `waiting`, `asked`). How `ASK`
reads when written in a piece's move (refused, as `ROLL` and `SHUFFLE` are). What the web board says
for `waiting` (slice 2). The shape of the saved state for a seat's kept answer.

## As built, slice 1: `IN_ORDER` (2026-11-25)

Built on `feature/ask-in-order`; as designed, with these differences and findings.

- **The state** gains `asked` (the seats still to answer, each with the stage it answers in) and
  `ask_then` (the stage the asker moves again in). Both are left out of a saved state when empty, so
  a game with no ask saves as before. `current_player()` is the first seat of `asked`; the cursor
  `current_turn` stays with the asker. The attack simulation clears `asked`, since it sets the cursor
  by hand.
- **`players`** is `OTHERS`, `ALL`, `MOVER` or a list of names. `MOVER` is new: it is how an
  answer asks the same seat again ("discard until seven"). The design's `MOVER's ENEMY` is not a
  form: this engine has no teams, so a two-player game's `OTHERS` is the enemy.
- **`mode`** is read as `IN_ORDER` and nothing else yet; `TOGETHER` and `sealed` are refused when the
  spec is read, as unknown, until slice 2.
- **An action's `stage`.** A stage that some action belongs to offers those actions alone; a stage
  no action belongs to offers the actions that name none, so every existing game plays as before.
  This is narrower than the design's wording ("an action without it is offered where it is now"),
  made so a reply stage is not also offered the asker's ordinary actions. An `ASK` needs an action
  that belongs to its stage.
- **A stage's `players`** is read for every stage: a player not listed has no way in it, which is how
  a seat is skipped.
- **Nested asks.** An answer's `ASK` puts its seats in front of those waiting and may not have a
  `then` (refused when read, in an action that belongs to a stage some `ASK` asks in): the first
  ask's `then` is the one that counts. An answer's `STAGE` keeps the same seat in that stage.
- **`LAST`** is the place where the way before ended for a seat that stays in a stage, and nothing
  in a `then` stage or a reply stage.
- **Dead stones** are marked by turning a stone into a piece `DEAD` (`SET_CODE`) rather than with
  `SET_FLAG`: a flag cannot be cleared, and `DISPUTE` has to give the stones back.
- **A finding.** `REPEATS` is refused in a game with `turns.stages`, so the dead-stone variant has no
  ko rule. The limit is older than `ASK`; it is documented in the known limitations.
- **Verification.** Engine: 21 tests in `tests/asks` (the order for each `players`, `then`, a
  skipped seat, every seat skipped, an answer that asks again, end checks waiting for the hand-on,
  a state saved mid-ask, a game with no ask unchanged, and what the spec refuses). Session
  `tests/dead_stones.rs`: six tests of Go's agreement over the session (the asked player is the one
  to move with only the answers offered, `AGREE` scores, `DISPUTE` gives the asker the marking again,
  `RESUME`, and the wrong player's answers refused). Mutation check: with the asked seat not the
  mover, seven of the 21 fail.
- **Review.** A review found that `eliminate_player` leaves `asked` alone: it only runs from an end
  check, judged when a turn is handed on, where nothing is asked, and resigning ends the match in the
  server, so it is noted in a comment and not changed. A state saved with `asked` or `ask_then` naming
  a player or stage the rules lack was accepted on resume and could not be played: it is now refused
  with the stage that does not fit (`stage_fits_the_rules`). An answer that asks again with no guard
  never ends, and an answer that decides the position is not seen by the end checks until the hand-on:
  both are documented in the spec format and not refused.

## As built, slice 2: `TOGETHER` (2026-11-25)

Built on `feature/ask-together`; as designed, with these differences and findings.

- **Playing as a seat (focus).** The state holds `together` (the stage, `sealed`, `then`, and for each
  seat its part-way answers and, once sealed, its kept answer). A seat answers by being *focused*:
  for the length of one answer the game is played as that seat, in the stage it is asked in, so the
  choices, ways, tests, `cancel` and the session's click feel are the ones any turn has. The design's
  `choose_as` exists as a convenience over `focus_seat`, `choose` and `unfocus`. `focus` is never
  saved. While the ask lasts nobody is the player to move and the asker's own ways are not offered.
- **A kept answer is the action and the answers, not the way.** A way is not saved; at the reveal each
  seat's ways are worked out again on the position, which has not changed (nothing is played until
  the last answer), and the kept answer finds its way. Each way is worked out before any is applied,
  so no answer is chosen on the effects of another. A restored state whose kept answer finds no way,
  whose seat is not in the turn order or whose stage is unknown is refused (`stage_fits_the_rules`).
- **`view.turn` is per viewer** while a seat owes an answer (the viewer, or else the first seat that
  does): so the web board's "your turn" and the server's choice for the caller needed no
  change, only the headline and a retry. The session gained `choose_as`, `cancel_as` and `is_to_move`;
  `choose` and `cancel` answer for the first seat that owes, which is the hot seat.
- **The server** asks `is_to_move(seat)` where it asked whose `turn` it was. The command log keeps a
  sealed answer, as it keeps every command, and is read by no endpoint.
- **The web board** names who it waits on, and sends an answer again once when a `409` came from the
  other seat answering first and its own answer was untouched (checked against what it is offered and the answers it has given).
- **Refused:** `sealed` without `TOGETHER`; `ROLL`, `SHUFFLE`, `STAGE` and `ASK` in an answer to an ask
  that asks all at once; an ask that asks all at once in an answer. (The design refused only `ROLL`
  and `SHUFFLE`; `STAGE` and `ASK` would need an answer to play half-way, so they are refused too.)
- **Open mode** drops the answers half given by the seats still to answer, and drops a seat with
  no way left.
- **Rock-paper-scissors** (now the catalogue game, `crates/engine/specs/rock_paper_scissors.json`, with `tests/rps.rs`; over HTTP in
  `crates/game/tests/play.rs`): a round is `ASK` of `ALL` (the asker is asked too), each throw goes
  into the thrower's own hidden bag, and twelve end checks read the two throws (each way round, for
  the player who just moved and the next). A drawn round leaves the match going with the next
  player to start the round, which first empties both bags.
- **Verification.** Engine: 16 tests in `tests/together`. Session `tests/rps.rs`: nine tests (what each
  viewer is told, a kept throw hidden until both are in, either order, every pair judged, a draw
  played again, a throw once and only when asked, a saved match resumed, the open mode, the hot seat,
  and 30 random matches saved and resumed after every answer and compared with a straight play).
  Game crate: the match over HTTP in either order, and two seats answering in the same moment (one
  applied, one `409`, the retry decides the round). Web: the headline and the retry. Mutation check:
  with the answers played at once (`sealed` ignored), two session tests and four engine tests fail.
- **Review.** A review found, and this fixes: the attack simulation kept the focused seat as the
  mover; a seat whose ways all start with one piece was not given that piece chosen, as every stage
  does (so cancel was refused with nothing chosen); the in-order resume test was dropped from `tests/asks`;
  the web retry compared only the answers given (it now compares the whole choice); and a stray doc
  line. Not changed: a kept answer that finds no way at the reveal is skipped (it cannot happen:
  nothing is played between the answer and the reveal, and a restored state is checked), and in the
  open mode an answer that decides the game does not end it until the others have answered, as in
  `IN_ORDER` (documented).
