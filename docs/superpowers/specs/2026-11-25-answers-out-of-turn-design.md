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
