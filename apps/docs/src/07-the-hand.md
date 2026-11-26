# 7. The hand: bags, drops and optional promotion

In every game so far, a captured piece left the game for good. In **Shogi** (Japanese chess) it
does not. You keep it, and on a later turn you may *drop* it back onto the board as one of your
own. This chapter introduces the **hand**, which the engine treats as a **bag**: a store of
pieces that are off the board. It also shows a more flexible form of promotion.

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
  a piece promote when it *leaves* that zone. A test could say so, by asking `IN_ZONE` about
  `START` too, but `shogi.json` only looks at where a move lands.)

## Declaring the hand: a bag

A place is somewhere on the board. A **bag** is somewhere off it, a store that keeps pieces as
counts: *three pawns and a gold*, with no order and no squares. The hand is the bag named `HAND`,
and the spec declares it at the top level:

```json
"bags": [ { "name": "HAND", "for": "EACH_PLAYER" } ]
```

`for` says whose bag it is. With `EACH_PLAYER`, each player has a bag of that name: their own
hand. A game with no `bags` has no hand, and removed pieces leave the game. The bags and what
they hold are part of the saved game state.

The name `HAND` is only a habit, and any bag can be used the same way. Chapter 12 has bags that
are not hands: a shared one, one with a capacity, and one that starts full.

(Older specs wrote `"hand": { "enabled": true }`. That form is gone, and the engine refuses it
as an unknown field. Write `bags`.)

## What goes into the hand: `removed_to`

Nothing goes into a bag by itself. A move's `MOVE` effect removes whatever stands on its
target, and you say where that piece goes by writing `removed_to` on the effect:

```json
{ "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" },
  "removed_to": { "bag": "HAND", "of": "MOVER" } }
```

`{ "bag": "HAND", "of": "MOVER" }` is the `HAND` of the player whose turn it is. `of` can also be
`ENEMY`, or a player's name, and is left out for a bag that is shared. A piece of the mover's own
is never put in their hand, so the field only matters for captures. A `REMOVE` effect takes the
same field, as Crazyhouse's en passant does. Written on every capturing effect, it is repetitive, and
that is the point: the engine has no rule about captures, and a game could keep only some of
the pieces it takes.

## Which piece it is: `captured_as`

If your opponent's promoted rook (a DRAGON) is captured, you should receive an ordinary ROOK, not
a DRAGON. The engine knows nothing about Shogi, so each *promoted* piece names the piece it turns
back into:

```json
{ "code": "DRAGON", "captured_as": "ROOK", "moves": [ ... ] }
```

A piece with no `captured_as` enters the bag as itself. The design pattern is: **every promoted
form is its own piece code**, with `captured_as` pointing back to its base. Shogi has several:
`DRAGON` (from `ROOK`), `HORSE` (from `BISHOP`), `TOKIN` (from `PAWN`), and one each for the
silver, knight and lance.

## Dropping pieces

A turn is now either *a move* or *a drop*, and each is an action. Shogi's `actions` list has two:
`MOVE`, the one you know, and `DROP`:

```json
{
  "name": "DROP",
  "do": [
    { "choose": "HELD", "in": { "bag": "HAND", "of": "MOVER" } },
    { "choose": "PLACE", "in": { "region": "BOARD" } },
    { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } } } },
    { "test": "ANY", "of": [
      { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "START" } }, "where": { "codes": ["PAWN"] } } },
      { "test": "ALL", "of": [
        { "test": "NOT", "of": { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "PROMO_FORCED" } },
        { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "LINE", "through": { "place": "TARGET" }, "along": [0, 1] }, "where": { "owner": "ALLY", "codes": ["PAWN"] } } }
      ] }
    ] },
    { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } }
  ]
}
```

(The real action has a rule like the `ANY` above for the lance and the knight too. They are left
out here.) It reads in order:

- **`{ "choose": "HELD", "in": { "bag": "HAND", "of": "MOVER" } }`** asks which held piece to
  drop, and from which bag. The options are the codes you hold there. `START` now names that
  piece, *the piece in the hand*. A `where` with `codes` limits the offer, as it does for a
  `PIECE` choice.
- **`{ "choose": "PLACE", "in": { "region": "BOARD" } }`** asks where. The `BOARD` region is every
  place of the board, and `TARGET` is the one picked.
- **The first test** asks for an empty square.
- **The `ANY`** is the pawn's rule. `ANY` holds when one of its tests does, so this reads: *either
  the piece is not a pawn, or both of these hold*. A `COUNT` at `START` sees the held piece, so its `codes` tell which piece is being dropped. Without that first test the pawn's rule
  would bind every piece.
- **Inside the `ALL`**, the first test keeps the pawn out of `PROMO_FORCED`, a zone (chapter 3)
  covering the last row for each player. Dropping there would leave a pawn that can never move.
  The second is the *nifu* rule. A `LINE` is every square on a line through a place, both ways, to
  the edges of the board. Through `TARGET` along `[0, 1]`, that is the drop square's whole file
  (column). The `COUNT` looks for your own pieces with the code `PAWN` on it, and `NOT` says there
  must be none. A promoted pawn is a `TOKIN`, so it does not count.
- **The `MOVE` effect**, with `START` held, takes one piece of that code out of the bag and puts a
  new piece on `TARGET`.

A `LINE` is an ordinary region: a move's tests can use it too. Its `along` is turned around
for each player like a `step`, and it skips any disabled squares on the way.

The knight and lance have their own versions of the zone (`PROMO_FORCED_KNIGHT` covers the last
two rows for the knight, which leaps two ahead).

Only the options that can finish are offered. A pawn is not offered on a file that already has
one of yours, and a drop that would leave your own king in check is never offered either: the
check entry of `every_action` (chapter 2) runs after every action, the drop included.

Choosing a held piece is an answer like any other. On the wire it is `{ "held": "GOLD" }` (the
engine still reads the old `{ "hand": "GOLD" }`), and in a live match it goes to the server, as
choosing a piece on the board does.

## Optional promotion

In chapter 3, promotion was forced: a soldier that reached the last row became a MANTRI without
being asked. Shogi offers a *choice* in some places, and none in others. Look at the end of the
SILVER's forward move:

```json
{ "if": { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "PROMO_FORCED" },
  "then": [{ "effect": "SET_CODE", "at": { "place": "TARGET" }, "code": "PROM_SILVER" }],
  "else": [
    { "if": { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "PROMO_ZONE" },
      "then": [
        { "choose": "CODE", "as": "promotion", "options": ["PROM_SILVER", "SILVER"] },
        { "effect": "SET_CODE", "at": { "place": "TARGET" }, "code": { "chosen": "promotion" } }
      ] }
  ] }
```

There are two cases, one inside the `else` of the other:

1. In the **forced** zone, the piece becomes `PROM_SILVER`. There is nothing to ask.
2. Otherwise, anywhere in the wider **promotion zone**, a `CODE` choice offers `PROM_SILVER` *or*
   `SILVER`. The player can promote, **or stay as they are**, by choosing the piece's own code.
   Anywhere else, nothing happens.

That is the whole trick for optional promotion: **include the piece's own code in the options**.
The `else` is why the forced zone does not also ask: the second `if` only runs where the first did
not hold.

You can see both in the CLI. A silver stepping into the promotion zone is asked to choose:

```text
Codes: PROM_SILVER, SILVER
```

and a pawn stepping onto the last row is simply turned into a `TOKIN`, with no question.

## What the engine does

- It keeps captured pieces in the bag you name, per player, demoted as you describe.
- It computes legal drop squares, respecting the drop action's tests and check.
- It places fresh pieces, so nothing carries over from the piece's earlier life.
- It asks for a promotion where the spec has a `CODE` choice, and lets players pick or decline.

## Try it

```sh
cargo run --bin modern-chess-cli crates/engine/specs/shogi.json
```

Make a scratch copy of `shogi.json` with a short game: SENTE has a KING at `[4, 0]` and a ROOK at
`[0, 0]`, and GOTE has a KING at `[8, 8]` and a GOLD at `[0, 5]`. Take the gold with the rook
(`0,0`, then `0,5`), and let GOTE move its king (`8,8`, then `8,7`). SENTE now has the gold in
hand, and the game says so:

```text
Current player: SENTE
Places: 0,5 (Piece), 4,0 (Piece)
Held: GOLD
Actions: MOVE, DROP
```

The places are the pieces that can move, and `Held` is the pieces that can be dropped. `MOVE`
and `DROP` are the two actions. Type `GOLD` to choose the held piece, and the places become
every empty point (`Place`): that is the `PLACE` choice, and any of them is a legal drop. Type
`cancel` to go back.

The web app plays Shogi as well, with optional promotion and drops. The tests are another way to
see the hand in action. In `crates/engine/src/tests/crazyhouse/mod.rs`,
`test_capture_then_drop_full_cycle` follows a capture all the way into the hand and back onto the
board.

## Now change it

1. **Change what a capture gives.** In `shogi.json`, change the DRAGON's `"captured_as": "ROOK"`
   to `"BISHOP"`. Capturing a dragon now puts a bishop in your hand. What would you change to
   make a captured *rook* give a bishop too?
2. **Mandatory promotion.** On the SILVER, remove `"SILVER"` from the `CODE` choice's options.
   Silvers must now promote whenever they enter the zone (and the game no longer asks).
3. **Crazyhouse.** Open `crazyhouse.json`. It is chess with the bag `HAND` and a `DROP` action. Find how a promoted pawn becomes `P_QUEEN`, and why the spec bothers to give it a new
   code. (Because of
   `captured_as`: a promoted queen must turn back into a pawn, but a native queen must not.)
4. **A hand that starts full.** In a scratch copy of `shogi.json`, give the bag a `start`:
   `{ "name": "HAND", "for": "EACH_PLAYER", "start": { "PAWN": 1 } }`. Both players now begin with
   a pawn to drop. Which test of the `DROP` action decides where it may go?
5. **A hand that cannot be filled.** Give the bag a `capacity` of `2`. What happens to a
   third capture? (A move that would overflow a bag is simply not offered. Chapter 12 explains.)

Next, effects: moves that change more than one square. Chapter 12 comes back to bags, and to
places that hold many pieces.
