# Rules that apply to every move (slice 3 of the generality pass)

Slice 3 of `2026-11-22-rule-blocks-design.md`. Slice 1 made tests blocks, slice 2 made a turn a
list of steps. This slice removes the rules that look at every move from the format: check, no
check on the opponent, forced captures, liberty capture and repeated positions.

## Why

Six switches on `Rules` each serve one family of games: `leader`, `no_opponent_check`,
`mandatory_capture`, `liberty_capture`, `forbid_repeated_positions` and, in the same code,
`allow_suicide`. A person or an agent writing a new game cannot reach any of them in pieces: "a
move may not leave my king attacked" exists only as the word `leader`, and "a group with no
empty neighbour is removed" only as `liberty_capture`. Since slice 2, an action's steps run in
order on the changing position, and a test step that fails removes that way of playing. Most of
these rules are therefore already sayable; they only need to be written once for every action.

## Decisions Frank made (2026-11-22)

- **Shared steps, not new hooks (approach B).** Rules on every move reuse actions' steps. The
  only new kind of rule is an obligation.
- **"No legal move" belongs to slice 4.** What happens to a player with no way to play
  (stalemate loses, checkmate, stalemate draws) is an outcome, so it is designed with the end
  checks. This slice only decides which ways exist.

## Goals

- `leader` (as a rule on moves), `no_opponent_check`, `mandatory_capture`, `liberty_capture`,
  `allow_suicide` and `forbid_repeated_positions` leave the spec format. Every spec that used
  them is rewritten and plays exactly as before.
- Every block added is used by more than one idea.
- Check detection and Go stay as fast as today (timing run within 5% of `main` on chess, Shogi
  and Go; identical move counts).

## Non-goals

- What happens when a player has no way (slice 4), and the check counter of Three-check (slice 4).
- Attack sets: how a piece's threats are worked out stays exactly as slice 2 left it. The two
  findings carried here (a slide over pieces threatens fewer squares than it reaches; a zone test
  does not limit threats, which Xiangqi's flying general relies on) move to slice 5, where the
  sets are rebuilt once per position. *(Claude's call: reworking threats changes which moves are
  legal in several games and belongs with the speed work.)*
- The jump lock (`continuing`): step 5 replaces it with turn stages.

## Anti-goals

- A block that only one game could use.
- A rule that silently means something else after the rewrite: an unknown action name, test or
  field is refused when the spec is read.
- Slower check detection for the sake of generality.

## Approaches

- **Shared steps plus one obligation block (chosen by Frank).** Rules on every move are steps
  appended to actions; new blocks are small and general.
- **Three new hook lists** (`refuse`, `must`, `after`) beside actions. A clean separate
  vocabulary, but it repeats "tests and effects", which actions already are. Ruled out by Frank.

## The blocks

### `every_action`

A top-level list of steps that run after an action's own steps, as if written at its end:

```json
"every_action": [
  { "for": ["STONE"], "do": [ ...steps... ] },
  { "do": [ ...steps... ] }
]
```

| Field | Meaning |
|---|---|
| `for` | Optional. The names of the actions it applies to. Absent means every action. |
| `do` | Steps: tests, effects and `if`. A `choose` is refused when the spec is read. |

Entries run in the order written, after the action's own steps and before the way is complete,
so a failed test removes the way exactly as one inside the action would. An action name that no
action has is refused when the spec is read. A pass (an action with no steps) is subject to an
entry only when it is named or the entry has no `for`.

### `must`

A top-level list of tests on a way. Each entry narrows the ways in the order written: if some
way passes the test, only the ways that pass stay; if none does, nothing changes.

```json
"must": [ { "test": "REMOVED", "where": { "owner": "ENEMY" } } ]
```

It runs on the ways that survived every test, and on every action, so a game with a forced
capture offers no drop or pass while a capture exists (the behaviour slice 2 gave
`mandatory_capture`).

### New tests, regions and filters

| Block | Written | Meaning |
|---|---|---|
| test `REMOVED` | `{ "test": "REMOVED", "where": F }` | The way has so far taken off the board a piece that matches `F` (`owner` is relative to the mover; `codes`; `moved`). It counts a piece removed by `REMOVE`, by a `MOVE` onto it, by a `PLACE` onto it, and one that goes to a hand. An enemy piece removed is slice 2's *capture*. |
| test `REPEATS` | `{ "test": "REPEATS" }` | The position as the steps so far leave it, with the turn handed to the next player and timed flags ticked, is one the game has already recorded after a turn (as the repetition check does today; the starting position is not recorded, so it does not count). A spec with a jump lock (`jump_enabled`) that uses `REPEATS` is refused when read: the jump lock re-works the ways in the middle of a turn, where "the turn handed on" is wrong. |
| region `BOARD` with `where` | `{ "region": "BOARD", "where": F }` | The places holding a piece that matches `F`. Without `where` it is every place, as in slice 2. |
| region `GROUP` | `{ "region": "GROUP", "of": P, "through": "orthogonal", "where": F }` | The pieces that join the piece at `P` through neighbours of the same owner matching `F`, `P` included; empty when `P` holds no such piece. `through` is as in `AROUND`. |
| region `AROUND` of a region | `{ "region": "AROUND", "of": R, "through": "orthogonal" }` | `of` may be a region instead of a place: the neighbours of its places that are not in it. |
| filter `empty` | `{ "empty": true }` | The place holds no piece. `COUNT` over a region with this filter counts empty places. It cannot be combined with other fields; the spec is refused if it is. |
| `ATTACKED` `by` | `"by": "MOVER"` | Attacked by the mover's own pieces. Alongside `ENEMY` from slice 1. |

`REMOVED` and `REPEATS` judge the way as it stands at that step, so inside an action they are
legal anywhere; they matter most in `every_action` and `must`. `FOR_EACH` may nest: the inner
loop's `EACH` replaces the outer's inside it, and each loop takes its region before running.

## Behaviour changes, named

- A capture in Go (a stone removed by liberty capture) now carries the capture mark in the
  history, because it is now a `REMOVE` in the way. Today it has none.
- The order of the stones a Go move removes is not part of the contract: the same stones leave,
  and their order inside the way's changes may differ from today's board order. If an existing
  test fixes the order, the test is respelled.

## What the six switches become

**Check (`leader`).** Appended to every action of chess-like games:

```json
{ "do": [
  { "test": "NOT", "of": { "test": "REMOVED", "where": { "owner": "ALLY", "codes": ["KING"] } } },
  { "test": "ANY", "of": [
    { "test": "NOT", "of": { "test": "ATTACKED",
        "in": { "region": "BOARD", "where": { "owner": "ALLY", "codes": ["KING"] } }, "by": "ENEMY" } },
    { "test": "REMOVED", "where": { "owner": "ENEMY", "codes": ["KING"] } } ] } ] }
```

The first test is Atomic's "a capture may not blow up your own king". The second is "your king
may not be left attacked, unless the way took an enemy king" (today's `move_is_safe`). With no
king on the board (a hand-built position) neither test can fail, which is today's behaviour. Four
players work as today: any enemy king taken is enough.

**No check on the opponent (`no_opponent_check`, Racing Kings).** A second closing test:
`NOT ATTACKED in BOARD where ENEMY KING by MOVER`.

**Forced captures (`mandatory_capture`, Checkers).** `"must": [ REMOVED where owner ENEMY ]`.
The jump lock stays as it is.

**Liberty capture and suicide (Go, `liberty_capture`, `allow_suicide`).** On the `STONE` action:

1. For each orthogonal neighbour of `TARGET` that holds an enemy stone whose `GROUP` has no empty
   neighbour (`COUNT` of `empty` in `AROUND` the group is 0), remove every stone of that group.
2. A test: the group of `TARGET` has an empty neighbour. (A game that allows suicide writes an
   `if` here that removes the group instead of refusing.)
3. `NOT REPEATS`, replacing `forbid_repeated_positions`.

`PASS` is not named, so a pass is never refused for repeating, as today. The order (captures,
then suicide, then repetition) is today's.

## Checked when a spec is read

An unknown action name in `for`; a `choose` inside `every_action`; `empty` combined with another
filter field; `GROUP` or `AROUND` of a region on a map with a label the board does not have; a
`must` entry that is not a test. The old keys `no_opponent_check`, `mandatory_capture`,
`liberty_capture`, `allow_suicide` and `forbid_repeated_positions` are refused with the same
`OldForm` as slice 2 used.

## `leader` for one more slice

`leader` stays in specs, read only by the no-legal-move outcome (checkmate against stalemate) and
Three-check's counter, which slice 4 replaces. It no longer filters any move. It is also the one
special form this slice leaves in the format. *(Consequence of putting "no legal move" in slice 4.)*

## The engine

- **Attack sets do not change, and neither does their one filter.** Today a king counts as
  attacked only if `capture_is_a_real_threat` agrees: where some move removes more than its
  target (Atomic), a capture that would also blow up the attacker's own king is no threat, which
  is why two Atomic kings may stand side by side. `ATTACKED` keeps that filter, with the same
  gate (it only costs anything in games where a move removes more than its target), and the
  filter keeps reading `leader` until slice 4. *(Claude's call, after the builder check.)*
- **`ATTACKED` is lazy.** The speed gate is firm. Today's check stops at the first attacker that
  reaches a king and builds no set. `ATTACKED` must do the same: find the places of its region
  first, then ask whether any attacker reaches one, stopping at the first that does. It must not
  build every threatened square of every opponent for every way.
- Each `every_action` entry is compiled onto its actions, and judged with short-circuiting like
  any step. The `ATTACKED` test over `BOARD where ALLY KING` finds the kings first and then asks
  the attack set of the position after the way, as `move_is_safe` does now.
- `REMOVED` is the flag `Way` already keeps as `captures`, generalised to a filter over what the
  way removed.
- `REPEATS` reuses the position key and the stored hashes.
- `must` replaces the `mandatory_capture` branch in `compute_ways`.
- `retain_leader_safe`, `retain_no_opponent_check`, `settle`, `settle_liberties` and the three
  `Rules` fields go. The jump lock stays.

## Proving it

- Every catalogue spec, every book board and every spec inside a test is rewritten; every
  existing test keeps what it checks. Move counts per game stay identical (`catalogue_counts`).
- New engine tests: each new block, each refusal above, check (a pin, a king move, taking an
  enemy king while attacked), Atomic's own-king rule, four-player check, Racing Kings' opponent
  check, Checkers' forced capture and its jump lock, and for Go a capture, a suicide, ko, and a
  pass that repeats a position. One test per rule showing it agrees with the old code on a
  position (the old code removed in the same slice).
- The timing run stays within 5% of `main` on chess, Shogi and Go.
- The book's chapters on forced captures, placing stones and the spec reference teach the blocks.
- In a browser: chess check and checkmate, Racing Kings, Checkers' forced jump, a Go capture.

## Left to the builder

- Type names and file layout; how `every_action` is compiled onto actions; error texts.
- How `GROUP` and the empty-neighbour count are computed without cloning the position.
- The order of commits.
