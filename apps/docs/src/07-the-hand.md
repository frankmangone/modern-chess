# 7. The hand: drops and optional promotion

In every game so far, a captured piece left the game for good. In **Shogi** (Japanese chess) it
does not. You keep it, and on a later turn you may *drop* it back onto the board as one of your
own. This chapter introduces the **hand**, and with it a more flexible form of promotion.

The spec is `crates/engine/specs/shogi.json`. The players are called SENTE and GOTE, and the
board is 9×9.

## The rules we are building

- When you capture a piece, it goes into **your hand**, and you can drop it later.
- A **promoted** piece that is captured goes into your hand as its *unpromoted* self.
- You may drop a piece on **any empty square**, with a few exceptions:
  - a pawn may not be dropped on a file where you already have an unpromoted pawn (*nifu*),
  - a piece may not be dropped where it could never move again, such as a pawn on the last row.
- Pieces **promote** when a move lands in the last three rows, but unlike Chaturanga the player
  can **choose not to**, except where staying would leave the piece stuck. (Real Shogi also lets
  a piece promote when it *leaves* that zone. A zone is only tested against where a move lands,
  so the spec does not model that.)

## Turning the hand on

One line at the top of the spec:

```json
"hand_enabled": true
```

It is `false` by default. With it set, every capture puts the captured piece in the capturer's
hand. The hand is part of the saved game state.

## What goes into the hand: `demotes_to`

If your opponent's promoted rook (a DRAGON) is captured, you should receive an ordinary ROOK, not
a DRAGON. The engine knows nothing about Shogi, so each *promoted* piece names the piece it turns
back into:

```json
{ "code": "DRAGON", "demotes_to": "ROOK", "moves": [ ... ] }
```

A piece with no `demotes_to` enters the hand as itself. The design pattern is: **every promoted
form is its own piece code**, with `demotes_to` pointing back to its base. Shogi has several:
`DRAGON` (from `ROOK`), `HORSE` (from `BISHOP`), `TOKIN` (from `PAWN`), and one each for the
silver, knight and lance.

## Dropping pieces

A turn is now either *a move* or *a drop*. The engine offers a drop when a player selects a piece
from their hand: it works out every square where that piece may legally go, the player picks one,
and a **fresh** piece is placed: no move count, no notes, as if it had never moved.

The squares where a drop is **forbidden** are described on the piece, in `drop_restrictions`.
Here is the Shogi pawn:

```json
{
  "code": "PAWN",
  "drop_restrictions": [
    { "condition": "PROMO_FORCED" },
    { "condition": "ALLY_ON_FILE", "piece": "PAWN" }
  ],
  "moves": [ ... ]
}
```

`drop_restrictions` is a list of conditions, and if **any** of them fires for a square, the piece
cannot be dropped there:

- **`PROMO_FORCED`** is a zone (a `POSITION` condition, as in chapter 3) covering the last row
  for each player. Dropping there would leave a pawn that can never move.
- **`ALLY_ON_FILE`** fires when you already have a piece with the given code on the same file
  (column). This is the *nifu* rule. It is the only condition that exists *just* for drops.

The knight and lance have their own versions of the zone (`PROMO_FORCED_KNIGHT` covers the last
two rows for the knight, which leaps two ahead).

The engine also does one thing for you: a drop that would leave your own leader in check is never
offered.

## Optional promotion

In chapter 3, promotion was forced: a soldier that reached the last row had to choose from the
list. Shogi offers a *choice*, and in some places no choice. Look at the SILVER's forward move:

```json
"modifiers": [
  { "action": "TRANSFORM", "conditions": [{ "condition": "PROMO_FORCED" }], "options": ["PROM_SILVER"] },
  { "action": "TRANSFORM", "conditions": [{ "condition": "PROMO_ZONE" }],   "options": ["PROM_SILVER", "SILVER"] }
]
```

There are two modifiers, in order:

1. In the **forced** zone, the only option is `PROM_SILVER`. The piece must promote.
2. Anywhere in the wider **promotion zone**, the options are `PROM_SILVER` *or* `SILVER`. The
   player can promote, **or stay as they are**, by choosing the piece's own code.

That is the whole trick for optional promotion: **include the piece's own code in the options**.

You can see both in the CLI. A silver stepping into the promotion zone is asked to choose:

```text
Select option from ["PROM_SILVER", "SILVER"]
```

and a pawn stepping onto the last row is told there is only one answer:

```text
Select option from ["TOKIN"]
```

The engine uses the first modifier whose conditions pass, so put the stricter one first.

## What the engine does

- It stores captured pieces per player, demoted as you describe.
- It computes legal drop squares, respecting `drop_restrictions` and check.
- It places fresh pieces, so nothing carries over from the piece's earlier life.
- It pauses promotions, and lets players pick or decline.

## Try it

> The command-line player does not yet support drops: it only lets you select pieces on the
> board. The engine does, and the Shogi and Crazyhouse tests exercise every step.

```sh
cargo run --bin modern-chess-cli crates/engine/specs/shogi.json
```

You can play Shogi on the board, including optional promotion. To see the hand in action, read
the tests. In `crates/engine/src/tests/shogi/mod.rs`, `test_nifu_pawn_drop_blocked_on_same_file`
shows the second-pawn rule and `test_execute_drop_places_piece_and_decrements_hand` shows a drop.
In `crates/engine/src/tests/crazyhouse/mod.rs`, `test_capture_then_drop_full_cycle` follows a
capture all the way into the hand and back onto the board. They use the engine's `CalculateDrops`
and `ExecuteDrop` steps, which a graphical app would call.

## Now change it

1. **Change what a capture gives.** In `shogi.json`, change the DRAGON's `"demotes_to": "ROOK"`
   to `"BISHOP"`. Capturing a dragon now puts a bishop in your hand. What would you change to
   make a captured *rook* give a bishop too?
2. **Mandatory promotion.** On the SILVER, remove `"SILVER"` from the `PROMO_ZONE` modifier's
   options. Silvers must now promote whenever they enter the zone.
3. **Crazyhouse.** Open `crazyhouse.json`. It is chess with `"hand_enabled": true`. Find how a
   promoted pawn becomes `P_QUEEN`, and why the spec bothers to give it a new code. (Because of
   `demotes_to`: a promoted queen must turn back into a pawn, but a native queen must not.)

Next, side effects: moves that change more than one square.
