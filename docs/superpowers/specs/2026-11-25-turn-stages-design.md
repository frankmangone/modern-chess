# Turn stages (step 5 of the route)

Last step of the generality pass. `jump_enabled` and the `END_TURN` effect are the two switches
left that know about one game (draughts). They are replaced by one idea, a **stage**, which also
gives Kalah its free turn.

## Intent

A turn is a list of ways, and today it can only be one way long, or, with `jump_enabled`, a chain
of captures by one piece. Frank's rule for the pass is that anything that can be built from
general blocks is built from them. So the engine learns one thing: *after a way, the same player
may move again, in a named stage that limits which ways they may play.* Draughts' multi-jump and
Kalah's free turn are two uses of it; a double move (Monster Chess) is already a turn order with
a name repeated and stays that.

## Goals, non-goals, anti-goals

Goals: one block replaces `jump_enabled` and `END_TURN`; Checkers plays exactly as now (same
moves, same jump lock, same click feel, within 5% of `main`); Kalah's free turn is written from
the blocks.

Non-goals: **the Flying Knight.** It is a double move (already writable as `["WHITE", "BLACK",
"BLACK"]`) whose check rule is "a king is in check when it can be taken within the opponent's two
moves", and "the double mover may step into check on the first move as long as the second gets
out". Both need a look two moves ahead, which a stage does not give. It stays in
`known-limitations.md` with that reason. Also not here: stages that last "while a test holds" as
a loop (a stage is entered by an effect, so a loop is a stage that enters itself); answers by
other players out of turn (step 8); dice and hidden information (steps 6 and 7).

Anti-goals: a block only draughts could use; an old spec that silently plays differently
(`jump_enabled` and `END_TURN` are refused, naming `stages`); slowing the hot path (a game with no
stage pays one `Option` check).

## The design

**Spec.**

```json
"turns": {
  "order": ["RED", "BLACK"],
  "stages": [
    { "name": "JUMPING",
      "only": { "test": "AND", "of": [
        { "test": "SAME", "a": { "place": "START" }, "b": { "place": "LAST" } },
        { "test": "REMOVED", "where": { "owner": "ENEMY" } } ] } }
  ]
}
```

- A stage has a `name` and an optional `only`: a test judged on every way the stage could play
  next (on the position it leaves, with `START`, `TARGET` and `EACH` as in an action, and `LAST`
  the place the previous way ended). A way that fails it is not offered. No `only` allows every way.
- The effect `{ "effect": "STAGE", "name": N }` says: when this way is played, the turn is not
  handed on; the same player moves again in stage N, if they have a way that passes its `only`.
  If they have none, the turn is handed on as usual. A way with no `STAGE` effect hands the turn
  on and leaves any stage. Where a way runs `STAGE` twice, the last one counts.
- **`LAST`** is a new place: where the latest `MOVE` or `PLACE` of the turn put a piece (for a
  `MOVE`, its `to`). Inside a way it is the way's own latest; in a stage's `only` it is where the
  way before ended. Where nothing has moved yet it names no place: an effect that uses it does
  nothing and a test of it fails, as `START` does with no piece chosen.
- **`SAME`** `{ "test": "SAME", "a": PLACE, "b": PLACE }` holds when both places are named and are
  the same place.

**Draughts.** `jump_enabled` goes. After a capturing jump that does not crown, the man's and the
king's jump end with `{ "effect": "STAGE", "name": "JUMPING" }`. The crowning jump simply does
not run it, so `END_TURN` is not needed and goes too. The jumping stage's `only` is above.

**Kalah.** After the sowing, `{ "if": { "test": "IN_ZONE", "at": { "place": "LAST" }, "zone":
"STORE" }, "then": [ { "effect": "STAGE", "name": "AGAIN" } ] }`, with a stage `AGAIN` that has no
`only`. A player who has no stone to sow has no way, the turn is handed on, and the end check
(`STUCK`) ends the game as before.

**Engine.** `Rules.stages` (name → `only`) replaces `jump_enabled`. `GameState.stage` (name and
`LAST`) replaces `continuing`, and is saved. `Way` loses `ends_turn` and `lands` and gains
`stage`, `start` and `last`. `compute_ways` applies the stage's `only` first and `must` after it
(as the jump lock came before `must`). `play_way` enters or leaves the stage. When every way of a
stage begins with the same piece (as the jump's do), that piece is already chosen and cancelling
is refused, as the jump lock does today; a stage whose ways start in many places (Kalah) leaves
the choice open and may be cancelled.

**Read-time refusals.** A stage name written twice; a `STAGE` effect naming no stage; a stage
`only` that uses a test an action's steps could not (the same rules as `must`); `REPEATS` in a
spec that has stages (a stage re-works the ways in the middle of a turn, where "the turn handed
on" is wrong; this is today's rule for `jump_enabled`, kept).

**Session, wasm, web.** The jump lock's wording ("finish the jump with the selected piece first")
becomes "finish the turn's stage with the selected piece first". `View` gains nothing: the
choice already shows the open ways.

## Rulings made while designing (Claude's calls, for Frank to read)

- A named stage in `turns.stages`, not an inline `only` on the effect: the state must save the
  stage and restore it mid-turn, and a name is something it can save.
- No test `STAGE` and no `required` flag on stages, because nothing here uses them. Where the
  Flying Knight needs them they come with it.
- Old saved states with `continuing` load without the lock; this cannot meet a real match, since
  the old spec (`jump_enabled`) is refused (as `hand_enabled` was in #77), so a match pinned to
  it has to be re-seeded.

## Approaches

1. **Named stages and a `STAGE` effect (chosen).** One block, saved by name, the lock and the free
   turn both fit.
2. `turns.again`: a test after every way. It cannot say "only this piece may continue" without a
   second block, and needs `LAST` and `SAME` anyway.
3. Keep `jump_enabled` and add a Kalah switch. Rejected: it is the one-switch-per-game the pass
   exists to remove.
