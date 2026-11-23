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
- When it reaches the last row, it becomes a **MANTRI**, with no choice to make.
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
      "do": [
        { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } } } },
        { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } },
        { "if": { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "REACH_END" },
          "then": [{ "effect": "SET_CODE", "at": { "place": "TARGET" }, "code": "MANTRI" }] }
      ]
    },
    {
      "id": 1,
      "step": [-1, 1],
      "do": [
        { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } }, "where": { "owner": "ENEMY" } },
        { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } },
        { "if": { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "REACH_END" },
          "then": [{ "effect": "SET_CODE", "at": { "place": "TARGET" }, "code": "MANTRI" }] }
      ]
    },
    {
      "id": 2,
      "step": [1, 1],
      "do": [
        { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } }, "where": { "owner": "ENEMY" } },
        { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } },
        { "if": { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "REACH_END" },
          "then": [{ "effect": "SET_CODE", "at": { "place": "TARGET" }, "code": "MANTRI" }] }
      ]
    }
  ]
}
```

### Different rules for moving and capturing

A move's tests say what it needs on its target. Look at the two kinds of move:

| Move | Step | First test | Meaning |
|------|------|------------|---------|
| 0 | `[0, 1]` | `NOT` `COUNT` at `TARGET` | straight ahead, if empty. An enemy in front **blocks** it. |
| 1 and 2 | `[-1, 1]` and `[1, 1]` | `COUNT` at `TARGET`, `"owner": "ENEMY"` | diagonal, **only** if there is an enemy to capture. |

The soldier never moves diagonally to an empty square and never captures straight ahead. Each
rule needs only the right test on the target. A piece that does different things on an empty
square and on an enemy one, such as a chess pawn, simply has two moves with the same step.

### Forward is relative

The soldier's steps are written once, as if walking **up**. The `direction` matrix you met in
chapter 1 turns them around for BLACK, so BLACK's soldier steps *down* the board and captures
diagonally *down*. The two diagonal captures, `[-1, 1]` and `[1, 1]`, are turned around in the
same way.

This is why `direction` matters now: from here on, pieces are no longer symmetric.

## Zones: naming a part of the board

"The last row" is a different row for each player. WHITE promotes on row 7, BLACK on row 0. We
describe it once, by name, in a top-level `zones` list:

```json
"zones": [
  {
    "name": "REACH_END",
    "places": {
      "WHITE": [[0, 7],[1, 7],[2, 7],[3, 7],[4, 7],[5, 7],[6, 7],[7, 7]],
      "BLACK": [[0, 0],[1, 0],[2, 0],[3, 0],[4, 0],[5, 0],[6, 0],[7, 0]]
    }
  }
]
```

This defines a **zone**:

- **`name`** is the name you will use elsewhere in the spec.
- **`places`** gives, for each player, the list of squares that count as "in the zone".

The squares in a zone are **absolute board addresses**. They are *not* flipped by `direction`.
That is exactly why the lists differ per player: each player has their own far edge.

A rule asks about a zone with the test `IN_ZONE`:

```json
{ "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "REACH_END" }
```

It holds when the place `at` is in the mover's part of the zone. `TARGET` is the square the move
**lands on**, so this reads *"this move ends inside REACH_END"*.

`IN_ZONE` is the second test you have now met, after `ATTACKED`. Zones are special because *you*
define them and give them a name. Name a zone the spec does not declare, and the spec is refused
when it is read, with a message that names the zone.

## Promotion: `if` and `SET_CODE`

A move's steps run in order, and each one sees the board as the steps before it left it. The last
step of every soldier move is an **`if`**:

```json
{ "if": { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "REACH_END" },
  "then": [{ "effect": "SET_CODE", "at": { "place": "TARGET" }, "code": "MANTRI" }] }
```

Read it as: *"If the piece is now in the REACH_END zone, it becomes a MANTRI."*

- **`if`** is a test. When it holds, the steps in `then` run. When it does not, nothing happens.
  (An `else` list, which this move does not need, runs in that case.)
- **`SET_CODE`** changes the piece at a place into another piece code. It keeps its owner and is
  otherwise a new piece. The code must exist in `pieces`.

The `if` comes after the `MOVE`, so `TARGET` holds the soldier by then. On any square outside
`REACH_END` the `if` does nothing.

There is nothing to choose, so the game does not ask. A promotion with one option is just a
`SET_CODE`. The soldier turns into a MANTRI as it arrives.

Notice that **all three** soldier moves end with the `if`. A step belongs to a single move, so the
capture moves need it too, or a soldier that captures its way to the last row would stay a
soldier.

When there are several options, the game has to ask. That is a **choice**, like the ones in
chapter 1, whose answer is a piece code. Chess lists four:

```json
{ "choose": "CODE", "as": "promotion", "options": ["QUEEN", "ROOK", "BISHOP", "KNIGHT"] },
{ "effect": "SET_CODE", "at": { "place": "TARGET" }, "code": { "chosen": "promotion" } }
```

The `CODE` choice asks the player for one of the `options`. `as` gives the answer a name, and the
effect after it uses it: `{ "chosen": "promotion" }` stands for whichever code the player picked.
The player picks after the move, so the pawn is already standing on the last row, as it does on a
real board, and the move cannot be taken back. You will try this in the exercises below.

## Winning

Chapter 2 gave us checkmate. Chaturanga adds a second way to win, and its `end_checks` now look
like this:

```json
"end_checks": [
  { "when": { "test": "COUNT", "in": { "region": "BOARD" },
              "where": { "owner": "ENEMY", "except": ["RAJA"] }, "min": 0, "max": 0 },
    "then": { "outcome": "WIN" } },
  { "as": "NEXT", "when": { "test": "STUCK" }, "then": { "outcome": "LOSE" } }
]
```

- **The first entry** is the bare king. It counts the enemy's pieces that are *not* the RAJA
  (`except` is a filter field like `codes`, the other way round), and when there are none, the
  player who just moved wins. Capturing everything else wins the game.
- **The second entry** is "no legal moves": `STUCK`, judged for the player about to play. In
  chess a stuck player whose king is safe draws. Here it is a **loss** in every case, so the
  entry needs no king and replaces chapter 2's pair. Checkmate is just one way of being stuck.

The next chapters look at these closely. For now, notice that you did not write any code for
either rule: both are tests you already know.

## What the engine does

Compared with the last chapter, the engine now also:

- steps soldiers forward and captures diagonally, for both players,
- runs the soldier's `if` after the move, and turns the soldier into a MANTRI when it lands in
  `REACH_END`,
- and ends the game when the enemy is bare.

## Play it here

This is a small board with two foot soldiers per side, one step from promotion. The shaded squares
are the `REACH_END` zone. Move a PADATI into the zone, and it becomes a MANTRI on the spot, with
no question asked. This is the `SET_CODE` step at work. (The full game is `crates/engine/specs/chaturanga.json`.)

<modern-chess-board spec="boards/promotion.json" skin="boards/chaturanga.skin.json">Playable board (needs JavaScript).</modern-chess-board>

## Try it

```sh
cargo run --bin modern-chess-cli crates/engine/specs/chaturanga.json
```

Choose the soldier at `[0, 1]`. It has a single move, `[0, 2]`: the soldiers are boxed in by
their own side and nothing is there to capture.

Playing a soldier all the way up takes a while, so to see promotion quickly, make a copy of
`chaturanga.json` with a short game: change `starting_positions` so that WHITE has only a RAJA at
`[4, 0]` and a PADATI at `[0, 6]`, and BLACK has a RAJA at `[7, 7]` and a RATHA at `[7, 6]`. (Give
BLACK a second piece. A lone RAJA is "bare", and WHITE would win the moment the move ends.) Then
move the soldier from `[0, 6]` to `[0, 7]`: choose the soldier, then the square.

```text
Places: 0,6 (Piece), 4,0 (Piece)
Answer (a place, a code, a held piece, an action, or cancel): 0,6
Selected: 0,6
Places: 0,7 (Move)
Answer (a place, a code, a held piece, an action, or cancel): 0,7
```

The game asks nothing more. The turn passes to BLACK, and the soldier is a MANTRI (the board draws
it as `MAN`).

## Now change it

1. **Offer a choice.** In each of the soldier's three moves, replace the `SET_CODE` step in the
   `then` with a choice and an effect that uses it:

   ```json
   { "choose": "CODE", "as": "promotion", "options": ["MANTRI", "RATHA"] },
   { "effect": "SET_CODE", "at": { "place": "TARGET" }, "code": { "chosen": "promotion" } }
   ```

   Run the short game again. After the move the game asks which, and you answer `MANTRI` or
   `RATHA`.
2. **Promote earlier.** Edit the `REACH_END` zone so that WHITE's list is row `6` instead of row
   `7` (and BLACK's is row `1` instead of `0`). Soldiers now promote one row early.
3. **Be more generous.** Change `exempt` to `["RAJA", "MANTRI"]`. Now the game ends as soon as
   the enemy has nothing left but a RAJA and MANTRI. Try the short game above, but give BLACK a
   MANTRI at `[7, 6]` instead of the RATHA. End checks are judged after every turn, so the
   game ends as soon as WHITE makes any move.

Next: chess. It has the same zones and promotions, plus a handful of special moves that need
tests and memory.
