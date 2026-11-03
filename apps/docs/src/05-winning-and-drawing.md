# 5. Winning and drawing

So far a game could end in two ways: checkmate, or a player with no legal move. This chapter
collects *every* way the engine can end a game, and shows that most of them are one or two lines
of the spec.

The examples are four chess variants. Each is almost identical to chess. Only the ending rules
differ, so open each file next to `chess.json` and compare:

| Game | File | How it ends differently |
|------|------|-------------------------|
| King of the Hill | `king_of_the_hill.json` | Walk your king to the centre to win |
| Three-check | `three_check.json` | Check the enemy king three times to win |
| Shatranj | `shatranj.json` | Leave the enemy a bare king to win |
| Racing Kings | `racing_kings.json` | Reach the far rank first. Giving check is forbidden |

## The order of things

After every move the engine asks, in this order:

1. **Did anyone just win** by one of the `win_conditions`?
2. **Is it a draw** by one of the `draw_conditions`?
3. **Is the player to move out of moves?** If so, checkmate, or stalemate (a draw or a loss,
   depending on `stalemate_loses`).

The first question that says yes ends the game. Everything below is a piece of one of these three
questions.

## Instant wins: `win_conditions`

`win_conditions` is a list at the top of the spec. Each entry has a `type`, and the player who
just moved wins if any entry fires. There are three types.

### `PIECE_IN_ZONE`: get there first

```json
"win_conditions": [
  { "type": "PIECE_IN_ZONE", "piece": "KING", "zone": "CENTER" }
]
```

This is **King of the Hill**: the player wins when their KING lands in the zone named `CENTER`.
The zone is declared exactly like `REACH_END` in chapter 3:

```json
{
  "code": "CENTER",
  "type": "POSITION",
  "check": {
    "WHITE": [[3, 3], [4, 3], [3, 4], [4, 4]],
    "BLACK": [[3, 3], [4, 3], [3, 4], [4, 4]]
  }
}
```

Both players share the same four squares, which is natural for a hill in the middle.
**Racing Kings** works the same way with a zone called `BACK_RANK` (row 7 for both players): the
first king to reach it wins. Because every zone is a separate list *per player*, you could just as
easily give each player a different target, as `REACH_END` does for promotion.

### `OPPONENT_BARE`: leave them with nothing

```json
{ "type": "OPPONENT_BARE", "exempt": ["SHAH"] }
```

You met this in chapter 3. This is **Shatranj**, where the SHAH is the king. The player wins
when the opponent has no pieces left except the exempt ones.

### `CHECK_COUNT`: count the checks

```json
{ "type": "CHECK_COUNT", "threshold": 3 }
```

**Three-check**: each time a move puts the enemy's leader in check, the engine adds one to a
counter it keeps for each player. At `threshold` checks, the player wins. You do not have to track
anything; the counter is part of the saved game state.

You can see these fire in the CLI with tiny positions. For King of the Hill, put one king a step
from the centre and move it in. For Three-check, a rook giving three checks in a row wins.

## Draws: `draw_conditions`

A draw is a different animal: nobody wins. These rules live in one optional block:

```json
"draw_conditions": {
  "repetition_count": 3,
  "fifty_move_halfmoves": 100,
  "fifty_move_pawn_codes": ["PAWN"],
  "insufficient_material": [
    ["KING"],
    ["KING", "BISHOP"],
    ["KING", "KNIGHT"]
  ]
}
```

Every field is optional, and a rule you leave out does not exist. Chess uses all four:

| Field | The game is drawn when... |
|-------|---------------------------|
| `repetition_count` | the same position (including whose turn it is) has appeared this many times |
| `fifty_move_halfmoves` | this many moves in a row have passed with no capture and no pawn move |
| `fifty_move_pawn_codes` | tells the engine **which** pieces count as "pawn moves" for the rule above |
| `insufficient_material` | **every** player's remaining pieces match one of these lists |

Two things to note:

- `fifty_move_pawn_codes` exists because the engine does not know what a "pawn" is. You tell it.
  If your game has no such piece, leave it out.
- `insufficient_material` is a list of lists. A player's pieces "match" a list when they are
  exactly those pieces, so `["KING", "BISHOP"]` means *a king and one bishop*. In chess, a lone
  king against a lone king is a dead draw, and the CLI agrees:

```text
Stalemate! It's a draw.
```

(The CLI uses the same wording for every kind of draw.)

Shogi uses `"repetition_count": 4`, because its repetition rule needs four occurrences. Atomic
Chess lists only a lone king as insufficient material. Each game picks what it needs.

## Running out of moves

Even with no `win_conditions` and no `draw_conditions` a game can end, when the player to move
cannot move. Two fields decide how:

```json
"leader": ["KING"],
"stalemate_loses": false
```

| Situation | What happens |
|-----------|--------------|
| No legal move, and the leader is attacked | **Checkmate.** The player loses. |
| No legal move, leader safe, `stalemate_loses` is `false` | **Stalemate.** A draw. |
| No legal move, leader safe, `stalemate_loses` is `true` | The player loses. |
| No `leader` at all | A leader is never attacked, so it follows the two stalemate rows: a draw, or a loss if `stalemate_loses` is `true`. |

`stalemate_loses: true` is what Chaturanga, Shatranj, Shogi, Xiangqi, Checkers and Ataxx use.
In chapter 1, MARCH had no leader and the default setting, so a jammed board was a draw. If you
add `"stalemate_loses": true` to MARCH, the player who gets stuck **loses** instead. (The CLI
announces it as `Checkmate! WHITE wins!`, whatever the reason.)

## A forbidden move: `no_opponent_check`

Racing Kings has one more rule: **you may not give check**. Instead of a special condition on
every move, there is a single switch at the top of the spec:

```json
"no_opponent_check": true
```

With it set, the engine removes any move that would leave an opponent's leader in check. Try a
copy of `racing_kings.json` with only a rook on `[3, 0]` and an enemy king on `[7, 7]`. The rook
can slide to `[4, 0]`, `[5, 0]` and `[6, 0]`, but `[7, 0]` is not offered, because from there it
would attack the king down the file.

It is all-or-nothing. It applies to every piece and every move in the game.

## Now change it

1. **A smaller hill.** In `king_of_the_hill.json`, edit `CENTER` so that it holds only
   `[4, 4]` for both players. The hill is now a single square.
2. **Two checks.** Change `"threshold": 3` to `2` in `three_check.json`.
3. **No draws by repetition.** Remove `"repetition_count"` from `three_check.json`'s
   `draw_conditions`. The other draw rules still work, but repeating moves is no longer a draw.
4. **Losing by being stuck.** Add `"stalemate_loses": true` to `apps/docs/examples/march.json`,
   and play it to the jam from chapter 1. Compare the ending with a draw.

Next, a game where the *shape* of the board and the pieces' freedom both matter: Xiangqi.
