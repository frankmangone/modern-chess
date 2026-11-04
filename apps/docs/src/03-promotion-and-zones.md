# 3. Promotion and zones

In chapter 2 every piece moved the same way in every direction. Now we add the first piece with
a real *forward*: the foot soldier. It moves one way, captures another way, and when it reaches
the far side of the board it turns into something better. That last part needs us to talk about
a *zone* of the board.

We finish building **Chaturanga**, so this chapter's spec is the real one:
`crates/engine/specs/chaturanga.json`.

## The rule

- The **PADATI** (foot soldier) steps one square **forward** onto an empty square.
- It captures one square **diagonally forward**, never straight ahead.
- When it reaches the last row, it becomes a **MANTRI**.
- You win by checkmate, or by capturing every enemy piece except the RAJA ("bare king").
- If you have no legal move at all, you **lose** (in chess it would be a draw).

## The soldier

```json
{
  "code": "PADATI",
  "name": "padati (foot soldier)",
  "moves": [
    {
      "id": 0,
      "step": [0, 1],
      "actions": [{"state": "EMPTY", "action": "MOVE"}],
      "modifiers": [{"action": "TRANSFORM", "conditions": [{"condition": "REACH_END"}], "options": ["MANTRI"]}]
    },
    {
      "id": 1,
      "step": [-1, 1],
      "actions": [{"state": "ENEMY", "action": "CAPTURE"}],
      "modifiers": [{"action": "TRANSFORM", "conditions": [{"condition": "REACH_END"}], "options": ["MANTRI"]}]
    },
    {
      "id": 2,
      "step": [1, 1],
      "actions": [{"state": "ENEMY", "action": "CAPTURE"}],
      "modifiers": [{"action": "TRANSFORM", "conditions": [{"condition": "REACH_END"}], "options": ["MANTRI"]}]
    }
  ]
}
```

### Different rules for moving and capturing

This is where splitting `actions` by `state` earns its keep. Look at the two kinds of move:

| Move | Step | `actions` | Meaning |
|------|------|-----------|---------|
| 0 | `[0, 1]` | only `EMPTY` → `MOVE` | straight ahead, if empty. An enemy in front **blocks** it. |
| 1 and 2 | `[-1, 1]` and `[1, 1]` | only `ENEMY` → `CAPTURE` | diagonal, **only** if there is something to capture. |

The soldier never moves diagonally to an empty square and never captures straight ahead. Each
rule needs only the right action for the right state.

### Forward is relative

The soldier's steps are written once, as if walking **up**. The `direction` matrix you met in
chapter 1 turns them around for BLACK, so BLACK's soldier steps *down* the board and captures
diagonally *down*. The two diagonal captures, `[-1, 1]` and `[1, 1]`, are turned around in the
same way.

This is why `direction` matters now: from here on, pieces are no longer symmetric.

## Zones: naming a part of the board

"The last row" is a different row for each player. WHITE promotes on row 7, BLACK on row 0. We
describe it once, by name, in a top-level `conditions` list:

```json
"conditions": [
  {
    "code": "REACH_END",
    "type": "POSITION",
    "check": {
      "WHITE": [[0, 7],[1, 7],[2, 7],[3, 7],[4, 7],[5, 7],[6, 7],[7, 7]],
      "BLACK": [[0, 0],[1, 0],[2, 0],[3, 0],[4, 0],[5, 0],[6, 0],[7, 0]]
    }
  }
]
```

This defines a **zone**, which is a *named condition* of type `POSITION`:

- **`code`** is the name you will use elsewhere in the spec.
- **`check`** gives, for each player, the list of squares that count as "in the zone".

Two details matter here:

- The squares in a zone are **absolute board addresses**. They are *not* flipped by `direction`.
  That is exactly why the lists differ per player: each player has their own far edge.
- A zone is tested against the square a move **lands on**. Name it in a move's `conditions` or
  in a modifier's `conditions`, and it means "this move ends inside the zone".

A zone is the second kind of condition you have now met, after `NOT_ATTACKED`. Zones are special
because *you* define them and give them a name.

## Promotion: `modifiers`

A **modifier** changes what a move does once it has been chosen. The only one so far is
`TRANSFORM`:

```json
"modifiers": [
  {
    "action": "TRANSFORM",
    "conditions": [{"condition": "REACH_END"}],
    "options": ["MANTRI"]
  }
]
```

Read it as: *"If the move ends in the REACH_END zone, the piece must turn into one of these
options."*

- **`conditions`** says when the modifier applies. Here, when the soldier lands in `REACH_END`.
  On any other square it does nothing.
- **`options`** is the list of piece codes the player can pick from. Every code must exist in
  `pieces`.

When the modifier fires, the game pauses and asks the player to choose, even when there is only
one option, as in Chaturanga. In the CLI it looks like this:

```text
Select option from ["MANTRI"]: MANTRI
```

If the list has several entries, the player picks one. Chess lists four (queen, rook, bishop,
knight). We will do the same in the next chapter.

Notice that **all three** soldier moves carry the modifier. The modifier belongs to a single
move, so the capture moves need it too, or a soldier that captures its way to the last row would
stay a soldier.

## Winning

Chapter 2 gave us checkmate. Chaturanga adds a second way to win, and the top of the spec now
looks like this:

```json
"leader": ["RAJA"],
"stalemate_loses": true,
"win_conditions": [
  { "type": "OPPONENT_BARE", "exempt": ["RAJA"] }
]
```

- **`win_conditions`** is a list of instant-win rules, checked after every move. Here,
  `OPPONENT_BARE` fires when the opponent has nothing left but exempt pieces. The `RAJA` is
  exempt, so capturing everything else wins the game.
- **`stalemate_loses`** changes what "no legal moves, but not in check" means. By default it is
  a draw. With `true`, the stuck player loses.

The next chapters look at both fields more closely. For now, notice that you did not write any
code for either rule.

## What the engine does

Compared with the last chapter, the engine now also:

- steps soldiers forward and captures diagonally, for both players,
- watches the `REACH_END` zone and pauses the game when a soldier gets there, then waits for the
  player's choice,
- and ends the game when the enemy is bare.

## Play it here

This is a small board with two foot soldiers per side, one step from promotion. The shaded squares
are the `REACH_END` zone. Move a PADATI into the zone, and the game pauses and asks what it
becomes. This is the `TRANSFORM` modifier at work. (The full game is `crates/engine/specs/chaturanga.json`.)

<modern-chess-board spec="boards/promotion.json" skin="boards/chaturanga.skin.json">Playable board (needs JavaScript).</modern-chess-board>

## Try it

```sh
cargo run --bin modern-chess-cli crates/engine/specs/chaturanga.json
```

Select the soldier at `[0, 1]`. It has a single move, `[0, 2]`: the soldiers are boxed in by
their own side and nothing is there to capture.

Playing a soldier all the way up takes a while, so to see promotion quickly, make a copy of
`chaturanga.json` with a short game: change `starting_positions` so that WHITE has only a RAJA at
`[4, 0]` and a PADATI at `[0, 6]`, and BLACK has a RAJA at `[7, 7]` and a RATHA at `[7, 6]`. (Give
BLACK a second piece. A lone RAJA is "bare", and WHITE would win the moment the move ends.) Then
move the soldier from `[0, 6]` to `[0, 7]`. The game asks you to choose, and the soldier becomes
a MANTRI.

```text
Select option from ["MANTRI"]: MANTRI
```

## Now change it

1. **Offer a choice.** Change the soldier's `options` to `["MANTRI", "RATHA"]` on all three
   moves. Run the short game again and promote.
2. **Promote earlier.** Edit the `REACH_END` zone so that WHITE's list is row `6` instead of row
   `7` (and BLACK's is row `1` instead of `0`). Soldiers now promote one row early.
3. **Be more generous.** Change `exempt` to `["RAJA", "MANTRI"]`. Now the game ends as soon as
   the enemy has nothing left but a RAJA and MANTRI. Try the short game above, but give BLACK a
   MANTRI at `[7, 6]` instead of the RATHA. Win conditions are checked after every move, so the
   game ends as soon as WHITE makes any move.

Next: chess. It has the same zones and modifiers, plus a handful of special moves that need
conditions and memory.
