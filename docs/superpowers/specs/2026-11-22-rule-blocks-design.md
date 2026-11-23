# Rules from elementary blocks (the generality pass)

## Why

The hub's direction is people proposing their own games, and later a game builder and agents
that help write specs. Today's rule vocabulary grew one game at a time: switches such as
`jump_enabled` or `no_opponent_check` exist for a single game, and a dozen move conditions are
variations of "look at the piece at some square". A person or an agent composing a new game
needs a small set of blocks that combine, not one switch per known game.

Frank asked for this pass on 2026-11-21 and chose, on 2026-11-22, to do it before step 4c
(places that hold many pieces), because 4c's new kind of move would otherwise be built on
today's vocabulary and rewritten afterwards.

## Decisions Frank made (2026-11-22)

- **Blocks only.** The engine and the spec format know only elementary blocks. Every special
  form (`leader`, `stalemate_loses`, `jump_enabled`, `EXPLODE`, `PIECE_NOT_AT`, …) is removed
  from the format, not kept as a shorthand. Familiar patterns may come back later as *recipes*
  in a builder that writes blocks into a spec; recipes are not part of this pass.
- **A uniform tree.** Blocks are written as a regular tree (`{ "test": "COUNT", "in": {...},
  "where": {...} }`), not as hand-friendly shortcuts, because a builder and agents will read and
  write specs. We write the specs by hand until then.
- **Full licence to rewrite.** Every spec, test, book chapter and board can be respelled.
  Nothing is in production, so old-format specs do not have to load.

## Goals

- Each slice below removes the special forms it covers and rewrites every spec that used them.
- After the pass, every rule in a spec is made of the blocks, and every block is used by more
  than one idea.
- Every catalogue game and every board in the book plays exactly as it does today.
- **Grid games stay as fast as they are**: the timing run (`crates/engine/examples/speed.rs`)
  stays within 5% of `main` on each game, compared side by side as in step 4b.

## Non-goals

- Recipes and the builder.
- New games. A slice may add a test game only to prove a block works.
- Places that hold many pieces, hands and supplies as places (step 4c). `hand_enabled`,
  `demotes_to` and `unlimited_supply` are replaced there.
- Turn stages (step 5). `jump_enabled` ("the same piece keeps capturing") is replaced there, by
  a stage, and `allow_pass` with it.

## Anti-goals

- A block that only one game could use.
- A spec that silently means something else after the rewrite: a test name, region, place or
  filter the engine does not know is refused when the spec is read, never read as "false".
- Slowing down the hot path (move generation and check detection) for the sake of generality.

## Approaches

- **Blocks only, uniform tree (chosen by Frank).** The engine has one vocabulary. Specs get
  longer; a builder will hide that.
- **Blocks plus shorthand that expands when read.** Shorter specs, one engine vocabulary, but two
  ways to write each rule. Frank ruled it out: recipes belong in the builder, not the format.
- **Readable blocks** (`{ "at": [0, 1], "is": "EMPTY" }`). Easier by hand, irregular for tools.
  Ruled out by Frank for the same reason.

## The route

Five slices, one pull request each, in this order. Each later slice gets its own short design
when it starts; the sketches below fix only what each one covers. Frank added slice 5 on
2026-11-22.

1. **Tests** (this document's detail, below): regions, piece filters, counts, logic. Replaces
   every move condition, `drop_restrictions`, zone `restrictions`, the repeat's `until`, the
   modifier conditions and the top-level `conditions` list (which becomes `zones`).
2. **Actions and effects** (Frank, 2026-11-22: a move is a general thing, and a single piece's
   move is one simple case). An action is what a player can do on their turn: the choices they
   make (a piece, a place, an option from a list; later a number in 4c, a card in step 7), the
   tests that allow it, and the effects it has. Effects are elementary: place, remove, move,
   set owner, set code, set flag, add to a counter, and "for each place in a region". Today's
   four kinds of turn become ordinary actions: a piece move (choose a piece, choose a square its
   pattern reaches; `step` and `repeat` become one way to list the squares), a drop (choose a
   piece in hand, choose a square), a pass (no choices, no effects) and a promotion (a last
   choice of code, then "set code"). Replaces the occupancy-keyed `actions` (`MOVE`/`CAPTURE`),
   `CONVERT`, `COPY_SOURCE`, `EXPLODE`, the side effect `CAPTURE` at an offset, `modifiers`,
   `allow_pass`, and the separate drop and promotion phases. The session, the wasm wrapper, the
   game service's commands, the web app and the board element move from select / move / drop /
   pass / transform to one protocol: the view lists the actions available and the choice being
   made, and each click answers it, with piece moves keeping today's piece-then-square feel. An
   action built from a move pattern compiles to today's generator, so grid speed stays. Designed
   with Frank when slice 1 is merged; it is the pass's central vocabulary decision. It makes 4c
   ("how many" is one more choice) and step 5 (a turn as a sequence of actions) smaller.
3. **Rules that apply to every move** (built; design `2026-11-22-every-move-rules-design.md`):
   filters on the position after a move (check, ko, suicide, Racing Kings' "no check"), effects
   after every move (liberty capture), and obligations ("if a move of this kind exists, only
   those"), written as `every_action` and `must`. Replaces `no_opponent_check`,
   `mandatory_capture`, `forbid_repeated_positions`, `liberty_capture` (with `allow_suicide`) and
   `leader` as a filter on moves. "Attacked" stays a block, and check keeps today's fast path.
   What happens when a player has no legal move moved to slice 4, which removed `leader` and
   `stalemate_loses` (the `STUCK` test of an end check now says it).
4. **Ends and counters** (built; design `2026-11-23-ends-and-counters-design.md`): one list of end
   checks, each a test with an outcome (the mover wins, a named player wins, the highest score
   wins, a draw), and named counters that events add to. Replaces every win condition type,
   `draw_conditions`, and the check and fifty-move counters, and, with what happens when a player
   has no legal move, `leader` and `stalemate_loses`: all four are gone from the format and are
   refused with `OldForm` naming `end_checks`.
5. **Lean state** (built; design `2026-11-23-lean-state-design.md`): the game state stops naming things by strings. Today a position is a
   `String`, a piece carries its code, its player and its flags as strings, the state is a map
   keyed by those strings, every candidate move clones its piece and square, and the attack set
   is rebuilt from scratch for every test and check that asks for it. After this slice:
   everything that only reads the state (judging tests, generating moves, finding attacks)
   borrows it and never copies; a candidate move records what it changes (squares to clear and
   fill, by node id, and the piece by its ids) instead of carrying a copy of the piece, and the
   piece is built only when the chosen move is applied; each position's attack sets are worked
   out once and lent by reference to every test and check that asks (Frank, 2026-11-22: prefer
   borrowing to copying). Squares are the board's node ids (as inside the board since step 2),
   and piece codes, players and flags are small interned ids, the compact names those change
   records use. Names stay at the edges (specs, the session view, saved games, the game
   service). Proven by the timing run getting faster on chess and Shogi with identical move
   counts, and by saved games still loading. Claude's call on the order: last in the pass,
   because slices 2–4 change what the state holds, and before step 4c, which reshapes places.

### Fixed in slice 4

Carried findings that slice 4 closed:

- The repetition check counted each piece's has-moved flag, so Pong hau k'i's threefold draw came
  after 18 plies instead of 12. The flag is now part of the position record only in a game where
  some test, filter or flag reads `moved` (castling, double steps), so Pong hau k'i draws after 12.
- A `PIECE_IN_ZONE` with no `piece` was accepted and never fired. It cannot happen now: the form
  is gone, and a zone test is an ordinary `SOME` over a filter.

Found in slice 4 and left as it is: `TERRITORY`, a score term, is a block only Go uses. It cannot
be built from regions without an "each region of" iteration that no other game asks for yet, so it
is the one exception to the anti-goal of a block that only one game could use (ruled in the slice
4 design).

### Carried into later slices

Found while building slice 1 or earlier steps, and left to the slice that owns them:

- **Slice 2 (actions and effects):** side effects do not refuse unknown fields; make every effect block as
  strict as the test blocks.
- **Left open by slice 5 (moved from slice 3):** a piece's attack set ignores its moves'
  conditions and `until`, so a slide that passes over pieces threatens fewer squares than it can
  reach, and a zone test on a move does not limit its threats (Xiangqi's flying general relies on
  that today). Reworking threats changes which moves are legal. Slice 5 kept legality exactly as it was (Frank,
  2026-11-23) and made the walk lazy (`visit_threats`, no sets), so the rework is now a small,
  separate change that waits for Frank's rulings.
- **Every slice:** move toward refusing unknown fields everywhere in a spec, which retires the
  placeholder fields that refuse the old keys today.
- **Rules still to design, with Frank** (from step 4b): building rules (Catan's distance rule,
  roads that must connect), sliding along a map's links, and links that differ per player.
- **For Frank to look at:** `janggi_cannon.json` lets a cannon capture an enemy cannon, which
  standard Janggi forbids, and its quiet slide carries a not-a-cannon test that can never fail.

## Slice 1: tests

### The blocks

A **test** is one of:

| Test | Written | True when |
|---|---|---|
| `ALL` | `{ "test": "ALL", "of": [T, ...] }` | every test in `of` holds (an empty list holds) |
| `ANY` | `{ "test": "ANY", "of": [T, ...] }` | at least one holds (an empty list does not) |
| `NOT` | `{ "test": "NOT", "of": T }` | `of` does not hold |
| `COUNT` | `{ "test": "COUNT", "in": R, "where": F, "min": 1, "max": n }` | the number of pieces in region `R` matching filter `F` is within `[min, max]`; `where` defaults to every piece, `min` to 1, `max` to no limit |
| `ATTACKED` | `{ "test": "ATTACKED", "in": R, "by": "ENEMY" }` | some place in `R` is attacked by an opponent of the mover; `by` is `ENEMY` (the only value in this slice) |
| `IN_ZONE` | `{ "test": "IN_ZONE", "at": P, "zone": "PALACE" }` | place `P` is in the mover's part of the named zone |

A **region** is one of:

| Region | Written | Places |
|---|---|---|
| `AT` | `{ "region": "AT", "at": P }` | the one place `P` (none when `P` is off the board) |
| `PATH` | `{ "region": "PATH", "from": P, "to": P, "ends": ["FROM", "TO"] }` | the places on the straight line between the two, without its ends unless `ends` names them. A straight line steps by -1, 0 or +1 on each axis, as today. Grids only |
| `AROUND` | `{ "region": "AROUND", "of": P, "through": "adjacent" }` | the neighbours of `P` through `adjacent`, `orthogonal`, or a map's label |
| `LINE` | `{ "region": "LINE", "through": P, "along": [0, 1] }` | every place on the board's line through `P` in that direction, both ways, `P` included. `along` is turned for each player like an offset (Claude's call: every vector in a move is written from the neutral view; no current game differs). Grids only |

A region **cannot be formed** when its place is off the board, or a `PATH` crosses a hole or the
edge. (A `PATH` whose line is not straight, or whose direction cannot be fixed when the spec is
read, is refused when the spec is read; added after the branch review.) `COUNT` and `ATTACKED` over such a region are false, and
`NOT` flips them as usual (Claude's call: it keeps today's results, where a missing path fails
`PATH_PIECE_COUNT`, so castling across a hole stays refused because its path `COUNT` fails, and
an off-board leg square still lets the horse pass through `NOT`).

A **place** is `{ "place": "START" }` (the moving piece's square) or `{ "place": "TARGET" }`
(where the move lands; for a sliding move, the square being judged), with an optional
`"offset": [dx, dy]`, written from the neutral player's view and turned for each player like a
step. Grids only for offsets. In a drop, `TARGET` is the square being dropped on, and `START` is
refused.

A **filter** (`where`) is an object whose fields must all hold, each optional:
`"owner": "ALLY" | "ENEMY" | "ANY"`, `"codes": ["PAWN", ...]` (any of them),
`"flag": "EN_PASSANT"`, `"moved": true | false` (has moved at least once).

### Where tests appear

- A move's `conditions` and an action's `conditions`: lists of tests that must all hold.
- A modifier's `conditions`: now any test, not only a zone name.
- A piece's `drop_conditions`: tests that must all hold for a drop on a square. It replaces
  `drop_restrictions`, whose entries blocked a drop when they held.
- A repeat's `until`: a test judged on the next square before stepping into it (`TARGET` is
  that square). Defaults to `COUNT` at `TARGET` (the square is occupied), today's `NOT_EMPTY`. A slide that never stops early writes an empty `ANY` (today's `"NEVER"`, which only worked because an unknown name read as false).
- The top-level `conditions` list becomes `zones`:
  `"zones": [{ "name": "PALACE", "places": { "RED": [...], "BLACK": [...] } }]`, with places
  written as today (coordinates on a grid, names anywhere). Win conditions keep naming zones.

### What goes

`DEPENDS_ON`, `PIECE_FIRST_MOVE`, `CHECK_STATE`, `NOT_ATTACKED`, `PATH_NOT_ATTACKED`,
`PATH_PIECE_COUNT`, `PIECE_AT`, `PIECE_NOT_AT`, `LEG_EMPTY`, `ALLY_ADJACENT_COUNT`,
`ALLY_ON_FILE`, conditions named after a zone, `absent_ok`, piece `restrictions` (write the zone
test on each move instead), `drop_restrictions`, the `STATE` condition type, and repeat `until`
as a string. For example:

| Today | As blocks |
|---|---|
| `LEG_EMPTY [0, 1]` | `NOT(COUNT in AT(START+[0,1]))` |
| `PIECE_NOT_AT [1, 0] CANNON` on a slide | `NOT(COUNT in AT(TARGET) where codes [CANNON])` |
| `PIECE_FIRST_MOVE [3, 0] absent_ok` | `NOT(COUNT in AT(START+[3,0]) where moved true)` |
| `DEPENDS_ON 0` (pawn double step) | `NOT(COUNT in AT(START+[0,1]))` |
| `ALLY_ON_FILE PAWN` (drop) | `NOT(COUNT in LINE(TARGET, [0,1]) where owner ALLY, codes [PAWN])` |
| piece `restrictions: PALACE` | `IN_ZONE(TARGET, PALACE)` on each of its moves |

### Checked when a spec is read

Refused with a readable error, in the style of today's: an unknown test, region, place or
filter field; an unknown zone; a piece code in `codes` that no piece has; `min` above `max`; an
offset, `PATH` or `LINE` on a map; `START` in a drop condition; `owner` or `by` values other than
those above; a label in `AROUND` that the board does not have.

### The engine

- Tests are built once per spec into a compiled tree with each offset already turned for each
  player (as conditions are today), and judged with short-circuiting.
- One judge serves moves, actions, modifiers, drops and repeats.
- `ATTACKED` uses the same attack sets as today's `NOT_ATTACKED`.

## Proving it

- Every catalogue spec, every board in the book and every spec written inside a test is
  rewritten; every existing test keeps what it checks.
- New engine tests: each test, region, place and filter kind, the logic tests' empty cases, each
  refusal above, and one test per row of the "As blocks" table showing the rewrite behaves as
  the old condition did.
- The timing run stays within 5% of `main` on chess, Shogi and Go.
- The book's chapters on conditions, zones, the hand and designing a game teach the blocks;
  `specs/SPEC_FORMAT.md` documents them and drops the removed forms.
- In a browser: one game per family still plays (chess with castling and en passant, Xiangqi,
  Janggi's cannon, Shogi drops, a solidarity move).

## Left to the builder

- Type names, file layout, how the tree is compiled and judged.
- Error texts, in the style of today's.
- The order of slices' commits, and how much of the book changes in which commit.
