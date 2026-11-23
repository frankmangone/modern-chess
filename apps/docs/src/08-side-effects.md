# 8. Effects

A move normally changes one thing: a piece goes from here to there, and maybe removes an enemy.
In chapter 4 you saw moves that did more: castling also moved a rook, and en passant also removed a
pawn. Every change a move makes is an **effect**, one step of the move's `do`, and a move may
have as many as it needs.

This chapter shows that effects can build whole games. We look at two:

- **Atomic Chess**: every capture causes an explosion.
- **Ataxx**: pieces clone themselves and convert their neighbours.

## The toolbox

You have already used four effects. Here is the full list, with the new ones at the bottom:

| Effect | What it does |
|--------|--------------|
| `MOVE` | Moves the piece at one place to another. Whatever stood there is removed (chapters 1, 4) |
| `REMOVE` | Takes the piece at a place off the board (chapter 4: en passant) |
| `SET_CODE` | Turns the piece at a place into another piece code (chapter 3: promotion) |
| `SET_FLAG` | Attaches a named note to the piece at a place (chapter 4) |
| `PLACE` | Puts a new piece of a given code on a place |
| `SET_OWNER` | Makes the piece at a place belong to the mover |
| `FOR_EACH` | Runs steps once for every place of a region, naming each place `EACH` |
| `END_TURN` | Ends the turn when this way is played (chapter 9) |

Each takes the place it works on, written as you have written places so far: `START`,
`TARGET`, with an optional `offset`. `FOR_EACH` adds a third, `EACH`.

### Where an effect lives

An effect is a step, so it can be written wherever a step can: in a move's `do`, in an action's
`do`, or inside an `if` (chapter 3). Steps run in order, and each sees the board as the effects
before it left it. A test after a `MOVE` asks about the board after the piece has moved. That is
how a promotion's `if` can ask where the piece now stands.

## Atomic Chess: `REMOVE` and `FOR_EACH`

**The rule.** Chess, plus one change: when a piece captures, it explodes. The capturing piece, the
captured piece, and every piece on the eight surrounding squares disappear. Pawns are the
exception: they survive an explosion unless they were the piece directly involved.

The spec is `crates/engine/specs/atomic.json`. The piece moves are exactly chess's. The only
difference is in the moves that capture. Here is the rook's capture to the right, a slide:

```json
{
  "id": 1,
  "step": [1, 0],
  "repeat": { "loop": true },
  "do": [
    { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } }, "where": { "owner": "ENEMY" } },
    { "effect": "REMOVE", "at": { "place": "TARGET" } },
    { "effect": "REMOVE", "at": { "place": "START" } },
    { "effect": "FOR_EACH", "in": { "region": "AROUND", "of": { "place": "TARGET" } },
      "do": [
        { "if": { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "EACH" } }, "where": { "codes": ["PAWN"] } } },
          "then": [{ "effect": "REMOVE", "at": { "place": "EACH" } }] }
      ] }
  ]
}
```

Read the steps in order:

1. **The test** asks for an enemy on the target. Moving to an empty square is another move of the
   rook, the one you know, which does not explode.
2. **`REMOVE` at `TARGET`** takes the captured piece.
3. **`REMOVE` at `START`** takes the capturing piece. It never moves: it explodes where it stands.
4. **`FOR_EACH`** visits the squares around the target, one by one. Its `in` is a region, any
   region (`AROUND` here, and `BOARD` and `LINE` work too). Its `do` runs once for each place of
   it, and inside `do` the place named `EACH` is the one being visited.
5. **The `if`** inside asks whether the piece on `EACH` is a pawn. When it is not, a `REMOVE` takes it.
   (Where `EACH` is empty or off the board the `COUNT` is false, so the `NOT` holds and the
   `REMOVE` has nothing to remove.)

A `FOR_EACH` takes its places before the first run starts, so what is removed along the way does
not change which places are visited. Its `do` may hold effects and `if`s, with `EACH` in their
tests, but no choices (the player is not asked inside it), and no test written bare. To test
inside a `FOR_EACH`, put the test in an `if`.

Set up a position with a white rook on `[0, 0]`, a black knight on `[0, 4]`, a black bishop on
`[1, 4]`, a black queen on `[1, 3]`, and a black pawn on `[0, 5]`. Play the rook to `[0, 4]`. The
rook, knight, bishop and queen are all gone, but the pawn is untouched. Your own pieces count too:
the capturing rook is destroyed by its own explosion.

### Leaders and explosions

The engine applies three rules whenever a game has a `leader` and a move can remove more than
what it lands on: a `REMOVE` at `START`, or a `REMOVE` inside a `FOR_EACH`, as the capture above
has. They are the real Atomic Chess rules, so the spec does not need to spell them out:

- A capture whose blast would destroy **your own leader** is never offered. That includes a
  capture that would destroy both leaders.
- A capture whose blast destroys **the opponent's leader** is always offered, even if your own
  leader is in check. The game ends on the spot.
- Two leaders may stand side by side. Neither is in check from a piece that could only take it
  by blowing up the capturer's own leader too.

Games whose moves never remove more than their target do not pay for any of this.

## Ataxx: `PLACE`, `MOVE` and `SET_OWNER`

**The rule.** Ataxx has a single kind of piece, a STONE, on a 7×7 board. On your turn you pick a
stone and either:

- **clone** it onto an adjacent empty square (the original stays put), or
- **jump** it two squares away (the original leaves).

In both cases, every enemy stone next to the landing square becomes yours.

The spec is `crates/engine/specs/ataxx.json`. It has no leader, and `stalemate_loses` is on, so
a player who cannot move loses. There is only one piece, but its moves use three new effects.

### A clone move

```json
{
  "id": 0,
  "step": [1, 0],
  "do": [
    { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } } } },
    { "effect": "PLACE", "at": { "place": "TARGET" }, "code": "STONE" },
    { "effect": "FOR_EACH", "in": { "region": "AROUND", "of": { "place": "TARGET" } },
      "do": [
        { "if": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "EACH" } }, "where": { "owner": "ENEMY" } },
          "then": [{ "effect": "SET_OWNER", "at": { "place": "EACH" } }] }
      ] }
  ]
}
```

Take it apart:

- The first test asks for an empty target, as a step does.
- **`PLACE`** puts a new stone on the target, owned by the player who moves. It is a new piece,
  not a copy, so it is unmoved. The original is not touched: it never left. That is a clone. (`PLACE` removes
  whatever stood on the place first, which is why the test comes before it.)
- **`FOR_EACH`** visits the eight squares around the target. The `if` asks whether the piece on
  `EACH` belongs to an enemy, and **`SET_OWNER`** makes that piece the mover's. If the square is
  empty, off the board or already yours, the `if` does nothing.

One `FOR_EACH` says "all the neighbours". You do not need an entry for each square.

### A jump move

A jump is a step of length two. It has the same `FOR_EACH` as a clone, but **no** `PLACE`: a
`MOVE` takes the stone itself.

```json
{
  "id": 8,
  "step": [2, 0],
  "do": [
    { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } } } },
    { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } },
    { "effect": "FOR_EACH", "in": { "region": "AROUND", "of": { "place": "TARGET" } },
      "do": [
        { "if": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "EACH" } }, "where": { "owner": "ENEMY" } },
          "then": [{ "effect": "SET_OWNER", "at": { "place": "EACH" } }] }
      ] }
  ]
}
```

With a `MOVE`, the source square is left empty and the stone has jumped. A stone has eight clone
moves (the adjacent squares) and sixteen jump moves (the squares at distance two), so the file is
long but regular. The engine does not mind: each one is an entry in `moves`.

### Playing it

```sh
cargo run --bin modern-chess-cli crates/engine/specs/ataxx.json
```

Make a scratch copy with one RED stone on `[0, 0]` and two BLUE stones on `[2, 0]` and `[1, 2]`.
Choose the red stone and you are offered `[0, 1]`, `[1, 0]` and `[1, 1]` (clones), and `[0, 2]`,
`[2, 1]` and `[2, 2]` (jumps). The squares with blue stones on them are not offered, because
the stone's first test asks for an empty target. Clone to `[1, 0]` and the blue stone on `[2, 0]`
turns red, while your stone on `[0, 0]` stays where it was. Jump to `[0, 2]` instead, and the
source square is left empty, and the blue stone on `[1, 2]`, right next to the landing square,
turns red.

## What the engine does

- It runs a move's effects in order, so the rest of the turn sees the new board. Converted stones
  really are yours when the engine decides what to do next.
- It lets any number of effects, and any number of `FOR_EACH`s, ride on one move.
- It counts a move for the piece you chose only (chapter 4). A stone that clones leaves the
  original's count as it was, and the new stone starts unmoved.

## Play it here

**Atomic Chess**, set up exactly as in the example above. Play the white rook up to the black knight
and watch the explosion: the rook, knight, bishop and queen all disappear, and the black pawn stays.

<modern-chess-board spec="boards/atomic_blast.json" skin="boards/chess.skin.json">Playable board (needs JavaScript).</modern-chess-board>

**Ataxx**, the whole spec. Click a stone, then clone it one square away or jump it two. Clone next
to an enemy stone, and the enemy stones touching your new stone change colour.

<modern-chess-board spec="boards/ataxx.json" skin="boards/ataxx.skin.json">Playable board (needs JavaScript).</modern-chess-board>

## Now change it

1. **Pawns burn too.** In `atomic.json`, on one capture move, delete the `if` and put its `REMOVE`
   straight in the `FOR_EACH`'s `do`. Pawns near that capture now die.
2. **The capturer survives.** Delete the `REMOVE` at `START` from the rook's capture. The rook
   captures and explodes, but is not removed: it stays where it stood.
3. **Jumps that do not convert.** Delete the `FOR_EACH` step of the jump move with
   `"step": [0, 2]`. Re-run the scratch game above and jump to `[0, 2]`: the blue stone on
   `[1, 2]` now stays blue. You have invented a variant in which only clones capture.
4. **Cloning for chess.** Give a chess piece of your choice a step after its `MOVE`:
   `{ "effect": "PLACE", "at": { "place": "START" }, "code": "ROOK" }` (with the piece's own code).
   What happens when a rook clones itself every time it moves?

Next: the last big rule family. Games in which you *must* capture.
