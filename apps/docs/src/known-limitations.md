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
- **Tests do not affect threats.** A move's tests and its `repeat.until` decide which
  moves are offered, not what a piece threatens. (The one exception is a test at the start of the
  move that asks for an empty `TARGET`: such a move threatens nothing.) A piece kept in a zone by `IN_ZONE` tests still
  threatens squares outside it. Xiangqi's flying general relies on this.
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
- **A game with stages cannot use `REPEATS`.** A stage works the ways out again in the middle
  of a turn, where "the turn handed on" has no meaning. A stage lasts one way: it is entered by a
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
  cannot use `REPEATS`, so that Go has no ko rule: the two do not yet go together.
- **A player who never answers blocks the game.** An asked player (`ASK`), like a player whose
  turn it is, can leave a match waiting for ever; resigning is the way out. Clocks will cover
  moves and answers together.
- **A placing action cannot be capped** (no "you have 12 stones left").
- **A dropped pawn may double-step from any rank.** A dropped piece has never moved, and the
  pawn's double step asks only that, and that the square ahead is empty. In standard Crazyhouse
  only a pawn on its second rank may double-step, so `crazyhouse.json` differs there.

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
