# Known limitations

Writing this guide meant trying every example against the engine. This page lists what
turned up. It describes how things behave **as of November 2026**. Some of these are rough
edges that will probably be fixed, so check them against the current engine before relying on
them.

## Sharp edges when writing a spec

- **Positions off the board** in `starting_positions` are ignored without warning. The piece
  just does not appear.
- **A `CODE` choice always asks.** There is no "skip". For optional promotion, list the piece's
  own code in `options`. A promotion with one option is not a choice: write a plain `SET_CODE`.
- **A held piece is chosen, not placed.** `HELD` picks a code from a bag, and `START` then
  names that piece: a test can look at its `codes`, and an offset from `START` cannot be formed.
- **What a piece attacks is judged by the leading tests of its moves only.** A piece attacks a
  place when one of its moves could take there, judged by the tests at the start of the move. A
  condition after an effect, a choice or an `if` is not read, and one that reads the way in progress
  or the attack map holds. The rules of `every_action` never apply to an attack, so a piece pinned
  to its king still attacks.
- **`COUNT` asks for at least one piece unless told otherwise.** Its `min` is `1` when left out,
  so `"max": 0` on its own is refused (`min` would be above `max`). Write `"min": 0, "max": 0`,
  or a `NOT` around the `COUNT`, for "no piece here".
- **A region that cannot be formed is false.** A place off the board, or a `PATH` that crosses a
  hole or the edge, makes `COUNT` and `ATTACKED` over it false. (A `PATH` that is not a straight
  line is refused when the spec is read.) `NOT`
  flips that, so "nothing stands on the square at `[0, 1]`" holds when that square is off the
  board.
- **A `LINE` does not stop at holes.** It skips disabled squares and runs on to the edge of the
  board. A `PATH`, by contrast, cannot cross a hole.
- **Offsets, `PATH` and `LINE` work on grids only.** A map refuses them; use `AROUND` there.
- **A game with stages can use `REPEATS` only in an end check.** A stage works the ways out again
  in the middle of a turn, where "the turn handed on" has no meaning, so a rule inside a way (a
  ko rule) cannot read it; an end check (a draw by repetition) is judged when the turn is handed on. A stage lasts one way: it is entered by a
  `STAGE` effect, and a loop is a stage that enters itself.
- **End checks are not judged between the ways of a stage.** They run when the turn is handed
  on, so a way that decides the game but enters a stage does not end it until the stage's last way.
- **The two-move check is exact only for pieces whose moves are their captures.** The Flying
  Knight (`flying_knight.json`) reads a king as in check when it can be taken within the
  opponent's two moves, with `ATTACKED` and `"moves": 2`. Only threatening moves count as the
  first move, so an attacker with pawns would be read approximately.
- **An end check does not keep a king safe.** Checkmate is the `STUCK` end checks of chapter 5, and
  they only say what a finished position means. A spec with no check entry in `every_action` lets
  a king walk into check. The old `leader` key is refused when the spec is read.
- **`TERRITORY` is a block only Go uses.** Counting the empty regions each player alone
  surrounds needs a way to repeat a rule for each region, and no other game asks for one yet.
- **`score` counts the board as it is.** The catalogue's Go does not let the players agree that
  a stone is dead, so it has to be captured before the passes. The score shown during play is the
  count as if the game ended at that moment, so early in a game it swings wildly. The agreement
  can be written (`ASK`, in the spec format's "Asking other players"), but a game with stages
  cannot use `REPEATS` inside a way, so that Go has no ko rule: the two do not yet go together.
- **A player who never answers blocks the game.** An asked player (`ASK`), like a player whose
  turn it is, can leave a match waiting for ever; resigning is the way out. Clocks will cover
  moves and answers together.
- **A placing action cannot be capped** (no "you have 12 stones left").
- **A piece dropped on the board is unmoved.** A spec that wants a dropped piece to count as moved
  (a rook that cannot castle) marks it with a flag when it is dropped and tests for the flag, as
  `crazyhouse.json` does. Its double step asks for the pawn's second rank instead of "has never
  moved", for the same reason.
- **`janggi_cannon.json` is Janggi cut down to its cannons.** It has a general and two kinds of
  piece, with no palace and none of Janggi's other pieces. Its cannon is the standard one: it
  moves only by jumping over exactly one piece, which may not be a cannon, and lands on an empty
  square or on an enemy piece that is not a cannon.
- **`4player_chess.json` is a four-player game cut down, not chess.com's four-player chess.** It is
  an 8×8 board with a king, a queen and two pawns each, not the 14×14 cross with sixteen pieces, and
  it has no points: a checkmated player is out, and the last one standing wins. It does follow what
  the standard rules say of a player who is out: their pieces stay on the board, a move can take
  them, and taking one earns nothing and does not count as a capture, so it is no help against a
  check.
- **A piece its owner moves into a bag is kept, not captured.** The "Captured" panel counts the
  pieces that left the board, except those their owner put into its own bag. A game that captures
  nothing turns the panel off with `"captured_panel": false` in its skin.

## What the bags and the maps of chapters 12 and 13 lack

- **A hidden bag is hidden in a match, not in "Try it".** A bag can say who sees it (`visible`: its
  owner, or nobody). The game server sends each seat, a spectator and a visitor only what they may
  see, and the web page draws a hidden bag as a stack of card backs with its count. "Try it" and the
  book show everything by design, since one person plays every side. The page draws a hidden bag
  as a stack and a count only: there is no "look at the top card" yet, and no per-card animation.
- **Dice are counts.** A roll adds pieces to a bag, so two dice are a total, not two dice.
- **A grid square holds one piece.** A game that needs a place with many pieces writes a map.
- **Links have no per-player form, and a piece cannot slide along a map's links.** The links of a
  label are the same for every player, and `repeat` is refused on a map. No game in the catalogue
  needs either yet (the Kalah of chapter 13 skips the other store with `skipping`).

## The command-line player

The CLI (`modern-chess-cli`) is a quick way to try a spec, not a finished product.

- It announces every win as `Checkmate! <player> wins!` and every draw as
  `Stalemate! It's a draw.`, whatever the real reason.
- It cannot draw a map: a game with a `"shape": "map"` board (chapters 12 and 13) stops it with
  an error. Play those through the web app, or read their tests.
- It does not say which of the places on offer capture: every place of a `MOVE` choice reads
  `(Move)`.
