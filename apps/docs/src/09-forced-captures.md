# 9. Forced captures and jumping

Our last game is **Checkers** (draughts). It looks easy, with one-square diagonal steps. But its
rules are about something we have not met: captures that **take the turn over**. You *must* capture
if you can, you may have to keep capturing with the same piece, and a crowning ends the turn.

The spec is `crates/engine/specs/checkers.json`. The players are RED and BLACK, the board is 8×8,
and each player starts with twelve pieces. It follows the standard English draughts rules, and
every one of them is written in the spec's blocks, not hidden in the engine.

## The rules we are building

- A **MAN** steps one square diagonally forward onto an empty square.
- It captures by **jumping**: over an adjacent enemy piece, onto the empty square just beyond.
- If **any** of your pieces can capture, you **must** capture.
- After a jump, if the same piece can jump again, it **must continue**, and the turn does not
  end until it cannot.
- A man that reaches the far row becomes a **KING**, which moves and captures in all four
  diagonal directions.
- A man that **jumps** onto the far row is crowned, and **the turn ends there**, even if the new
  king could jump again.
- A player with no legal move **loses**. (There is no king: no check, no checkmate.)

## A jump is a two-square step with tests and effects

Look at the MAN's capture to the forward right:

```json
{
  "id": 2,
  "step": [2, 2],
  "do": [
    { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } } } },
    { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START", "offset": [1, 1] } }, "where": { "owner": "ENEMY" } },
    { "effect": "REMOVE", "at": { "place": "START", "offset": [1, 1] } },
    { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } },
    { "if": { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "REACH_END" },
      "then": [
        { "effect": "SET_CODE", "at": { "place": "TARGET" }, "code": "KING" }
      ],
      "else": [
        { "effect": "STAGE", "name": "JUMPING" }
      ] }
  ]
}
```

Read it from the top:

1. **`step: [2, 2]`.** The piece lands two squares diagonally away.
2. **The first test** asks for an *empty* landing square.
3. **The second test**, a `COUNT`, asks for an enemy on the square between, at `[1, 1]` from the
   start. The filter's `owner` says *whose*: `"ENEMY"`, `"ALLY"` or `"ANY"`. The filter can also
   take `codes` (chapter 4), and a `NOT` around the `COUNT` gives the opposite.
4. **`REMOVE`** takes the jumped piece at `[1, 1]`, the same effect that en passant used in
   chapter 4.
5. **`MOVE`** puts the man on the landing square.
6. **The `if`** crowns the man when it lands on the last row, as in Chaturanga: `SET_CODE`
   turns it into a KING. Its `else` is the new part: the `STAGE` effect, below, which keeps the
   turn going when the man did not crown.

Why is this a capture, when the landing square is empty? Because a **capture** is any move that
removes an enemy piece, however it does so: with `REMOVE`, by a `MOVE` onto it, or by a `PLACE` on
it. The `REMOVE` takes an enemy, so the jump is a capture. The two rules below look for exactly that.
A plain step removes nothing, so it is not.

The KING has eight moves: four one-square steps, and four jumps in the same shape as above, but
without the crowning `if` (a king is already a king): each ends with the `STAGE` effect alone. The
MAN has two steps (forward only) and two forward jumps. The man's steps end in a crowning too, but
a shorter one: `SET_CODE` with no `else`, because a step ends the turn anyway.

## `must`: you must capture

```json
"must": [ { "test": "REMOVED", "where": { "owner": "ENEMY" } } ]
```

`must` is a list of tests at the top of the spec, and each one is an obligation: if *some* way of
playing the turn passes the test, only the ways that pass are offered. If none does, the test
asks for nothing and every way stays.

The test here is `REMOVED`. It holds for a move that took off the board a piece passing its
filter, and with `"owner": "ENEMY"` (seen from the player moving) that is a capture. So the line
reads: *"if you can capture, you must."* Once a player's turn begins, the engine looks for
captures across **all** of their pieces. If it finds even one, every move that is *not* a capture
is hidden, for every piece.

Try it with a scratch copy of `checkers.json`: RED men on `[0, 0]` and `[2, 2]`, BLACK men on
`[3, 3]`, `[5, 5]` and `[7, 1]`. The man on `[0, 0]` could normally step to `[1, 1]`. But the man
on `[2, 2]` can jump `[3, 3]`, so the only piece on offer at the start of the turn is the one that
can capture:

```text
Current player: RED
Places: 2,2 (Piece)
```

The man on `[0, 0]` is not offered at all. Choose `[2, 2]` and the jump to `[4, 4]` is the only
move.

Leave `must` out and a player is free to ignore a capture. (Rule books for Checkers differ on
this, which is exactly why it is written in the spec.)

## A stage: keep jumping

```json
"turns": {
  "order": ["RED", "BLACK"],
  "stages": [
    { "name": "JUMPING",
      "only": { "test": "ALL", "of": [
        { "test": "SAME", "a": { "place": "START" }, "b": { "place": "LAST" } },
        { "test": "REMOVED", "where": { "owner": "ENEMY" } } ] } }
  ]
}
```

So far a turn was one way. A **stage** lets the same player move again before the turn passes.
It has two parts. In `turns`, `stages` lists the stages by name, and a stage may have an `only`
test. In a move, the effect `{ "effect": "STAGE", "name": "JUMPING" }` says: *when this way is
played, do not hand the turn on. The same player moves again, in this stage.* Each jump ends with
it (except the crowning one, below). A way with no `STAGE` effect hands the turn on, as it always
did.

The stage's `only` says which ways may be played next. It is judged on each way the stage could
play, on the position that way leaves. Here it holds two things at once (`ALL`):

- `SAME` with `START` and `LAST`: the piece that moves is the one that has just landed. `START`
  is the piece of the way being judged. `LAST` is a new place: **where the latest `MOVE` or
  `PLACE` of the turn put a piece**. In an `only` it is where the way before ended. `SAME` holds
  when both places are named and are the same place.
- `REMOVED`: the way captures, the same test `must` uses.

So after a jump, only the piece that landed may move, and only to capture. If it has no capture
left, no way passes `only`, and the turn is handed on. If it has one, the same player moves
again, with that piece already chosen:

```text
Current player: RED
Chosen so far: [Place(Position("4,4"))]
Selected: 4,4
Places: 6,6 (Move)
```

While this lasts you cannot cancel: when every way of a stage starts with the same piece, that
piece is already chosen, and the jump has been made. Following the position above, the man jumps
`[2, 2]` → `[4, 4]` and is asked to continue. It jumps again to `[6, 6]`. Both black men are gone,
and then the turn passes.

Without the `STAGE` effect, the turn passes right after the first capture.

`must` and a stage work together but are not the same. `must` applies to every action (in a game
with drops, a drop would be hidden while a capture exists), and its test says exactly what is
forced. A stage's `only` narrows the ways first, then `must` narrows what is left. They are a few
lines of a spec, and they describe most of what separates draughts from chess.

## Crowning ends the turn

There is one more rule of draughts that a stage alone gets wrong. A man that jumps onto the last
row is crowned, and in English draughts **the turn ends there**, even if the new king could jump
again. A `STAGE` effect after the crowning would offer the king's next capture, so the jump keeps
it out of that case. The crown goes in the `then` of the `if` and the continuation in its `else`:

```json
{ "if": { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "REACH_END" },
  "then": [ { "effect": "SET_CODE", "at": { "place": "TARGET" }, "code": "KING" } ],
  "else": [ { "effect": "STAGE", "name": "JUMPING" } ] }
```

You saw it at the end of the man's jump above. It needs no block of its own: the crowning way
runs no `STAGE` effect, so the turn passes. (The king's jumps have no crowning, and just end with
the `STAGE` effect.)

The crowning jump is still a capture, so it follows every capture rule:

- it is **forced** like any other jump, by `must`;
- it **may end** a chain of jumps: a man that jumps along a chain and lands on the last row is
  crowned, and the turn ends.

A stage is not only for jumps. The Kalah of chapter 13 gives a free turn the same way, with a
stage that has no `only`, and a `STAGE` effect after a sowing that ends in your own store.

## What the engine does

- It treats any way that removes an enemy piece as a capture, and uses that for both rules.
- It keeps only the captures when one exists, as `must` says.
- It keeps the turn going after a way that runs `STAGE`, offering only the ways the stage's `only`
  allows, which locks the moving piece between jumps. The turn passes when a way does not run
  `STAGE`, or when no way passes `only`.
- It ends the game when a player has no move, because the spec's one end check is `STUCK` with
  the outcome `LOSE`.

## Play it here

The full draughts spec, running live. Move a man toward the enemy until a jump is possible. From then on
the board only lets you capture (`must`), and after a jump the same piece stays chosen if
another jump is available (the `JUMPING` stage).

<modern-chess-board spec="boards/checkers.json" skin="boards/checkers.skin.json">Playable board (needs JavaScript).</modern-chess-board>

## Try it

```sh
cargo run --bin modern-chess-cli crates/engine/specs/checkers.json
```

In the starting position nobody can capture, so every move is an ordinary step. Make a scratch
copy with the position above to see forced captures and chain jumps.

## Now change it

1. **Free choice.** Delete the `"must"` line. Rerun the position above. Both men are offered
   now, and choosing the man on `[0, 0]` offers `[1, 1]`.
2. **One jump per turn.** Remove the `STAGE` effect from the jumps (the `else` of the man's, the
   last step of the king's). After the first capture to `[4, 4]` the turn passes to BLACK, and the
   second man is safe.
3. **Backwards captures.** In international draughts, men may also capture backwards. Add two moves
   to the MAN:

   ```json
   { "id": 4, "step": [ 2, -2], "do": [
       { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } } } },
       { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START", "offset": [ 1, -1] } }, "where": { "owner": "ENEMY" } },
       { "effect": "REMOVE", "at": { "place": "START", "offset": [ 1, -1] } },
       { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } } ] },
   { "id": 5, "step": [-2, -2], "do": [
       { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } } } },
       { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START", "offset": [-1, -1] } }, "where": { "owner": "ENEMY" } },
       { "effect": "REMOVE", "at": { "place": "START", "offset": [-1, -1] } },
       { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } } ] }
   ```

   A man on `[4, 4]` with an enemy on `[3, 3]` is now offered the jump back to `[2, 2]`.
4. **Crowning does not end the turn.** In a man's jump, take the `STAGE` effect out of the `else`
   and put it after the `if`, so it runs for the crowning jump too. If the new king can jump again,
   it must: the turn goes on, as in some other draughts rules.

That's every part of the spec. The last chapter is about putting them to use on a game of your own.
