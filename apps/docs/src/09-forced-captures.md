# 9. Forced captures and jumping

Our last game is **Checkers** (draughts). It looks easy, with one-square diagonal steps. But its
rules are about something we have not met: captures that **take the turn over**. You *must* capture
if you can, and you may have to keep capturing with the same piece.

The spec is `crates/engine/specs/checkers.json`. The players are RED and BLACK, the board is 8×8,
and each player starts with twelve pieces.

## The rules we are building

- A **MAN** steps one square diagonally forward onto an empty square.
- It captures by **jumping**: over an adjacent enemy piece, onto the empty square just beyond.
- If **any** of your pieces can capture, you **must** capture.
- After a jump, if the same piece can jump again, it **must continue**, and the turn does not
  end until it cannot.
- A man that reaches the far row becomes a **KING**, which moves and captures in all four
  diagonal directions.
- A player with no legal move **loses**. (There is no leader: no check, no checkmate.)

## A jump is a two-square step with a condition

Look at the MAN's capture to the forward right:

```json
{
  "id": 2,
  "step": [2, 2],
  "actions": [{ "state": "EMPTY", "action": "CAPTURE" }],
  "conditions": [{ "condition": "PIECE_AT", "position": [1, 1], "player": "ENEMY" }],
  "side_effects": [{ "action": "CAPTURE", "target": [1, 1] }],
  "modifiers": [
    { "action": "TRANSFORM", "conditions": [{ "condition": "REACH_END" }], "options": ["KING"] }
  ]
}
```

Read it from the top:

1. **`step: [2, 2]`.** The piece lands two squares diagonally away.
2. **`actions`: `EMPTY` → `CAPTURE`.** It can only land on an *empty* square. That is unusual:
   until now `CAPTURE` always went with `ENEMY`, because the captured piece was the one on the
   landing square. Here, the piece that gets captured is somewhere else.
3. **`conditions`: `PIECE_AT`.** The square between, at `[1, 1]` from the start, must hold an
   enemy piece. `PIECE_AT` checks one square for a piece. Its `player` field says *whose*: `"ENEMY"`, `"SELF"`
   or `"ANY"`. It can also take a `piece` code (see the reference), and it has an opposite,
   `PIECE_NOT_AT`.
4. **`side_effects`: `CAPTURE`.** Removes the jumped piece at `[1, 1]`, the same side effect that
   en passant used in chapter 4.
5. **`modifiers`.** Landing on the last row promotes to a KING, exactly as in Chaturanga.

Why label the action `CAPTURE` at all, when the landing square is empty? Because the label is
how the engine *knows* a move is a capture. The two flags below look for it.

The KING has eight moves: four one-square steps, and four jumps in the same shape as above, one for
each diagonal. The MAN has two steps (forward only) and two forward jumps.

## `mandatory_capture`: you must capture

```json
"mandatory_capture": true
```

Once a player's turn begins, the engine looks for captures across **all** of their pieces. If it
finds even one, every move that is *not* a capture is hidden, for every piece.

Try it with a scratch copy of `checkers.json`: RED men on `[0, 0]` and `[2, 2]`, BLACK men on
`[3, 3]`, `[5, 5]` and `[7, 1]`. The man on `[0, 0]` could normally step to `[1, 1]`. But the man
on `[2, 2]` can jump `[3, 3]`, so selecting `[0, 0]` offers nothing at all. Select `[2, 2]` and the
jump to `[4, 4]` is the only move.

The flag is off by default. With it off, a player is free to ignore a capture. (Rule books for
Checkers differ on this, which is exactly why it is a flag.)

## `jump_enabled`: keep jumping

```json
"jump_enabled": true
```

When a piece captures, the engine checks whether the same piece, from where it landed, can capture
again. If so, the turn does **not** pass. The player is told:

```text
Continue jumping! Available captures:
[6, 6]: "CAPTURE"
```

While this lasts, only that piece may move, and only to capture. Following the position above,
the man jumps `[2, 2]` → `[4, 4]` and is asked to continue. It jumps again to `[6, 6]`. Both black
men are gone, and then the turn passes.

With `jump_enabled` off, the turn passes right after the first capture.

Both flags are **all-or-nothing**, like `no_opponent_check` in chapter 5: they apply to every
piece. They are exactly two lines of a spec, and they describe most of what separates draughts
from chess.

## What the engine does

- It treats any move whose action is `CAPTURE` as a capture, and uses that for both flags.
- It filters out non-captures when `mandatory_capture` applies.
- It **locks** the moving piece between jumps and keeps the turn going until the chain ends.
- It ends the game when a player has no move, because `stalemate_loses` is `true` and there is
  no leader.

## Try it

```sh
cargo run --bin modern-chess-cli crates/engine/specs/checkers.json
```

In the starting position nobody can capture, so every move is an ordinary step. Make a scratch
copy with the position above to see forced captures and chain jumps.

## Now change it

1. **Free choice.** Set `"mandatory_capture": false`. Rerun the position above. Selecting the man
   on `[0, 0]` now offers `[1, 1]`.
2. **One jump per turn.** Set `"jump_enabled": false`. After the first capture to `[4, 4]` the turn
   passes to BLACK, and the second man is safe.
3. **Backwards captures.** In international draughts, men may also capture backwards. Add two moves
   to the MAN:

   ```json
   { "id": 4, "step": [ 2, -2], "actions": [{"state": "EMPTY", "action": "CAPTURE"}],
     "conditions": [{"condition": "PIECE_AT", "position": [ 1, -1], "player": "ENEMY"}],
     "side_effects": [{"action": "CAPTURE", "target": [ 1, -1]}] },
   { "id": 5, "step": [-2, -2], "actions": [{"state": "EMPTY", "action": "CAPTURE"}],
     "conditions": [{"condition": "PIECE_AT", "position": [-1, -1], "player": "ENEMY"}],
     "side_effects": [{"action": "CAPTURE", "target": [-1, -1]}] }
   ```

   A man on `[4, 4]` with an enemy on `[3, 3]` is now offered the jump back to `[2, 2]`.

That's every part of the spec. The last chapter is about putting them to use on a game of your own.
