# Generalising the engine

## Why

The engine plays deterministic, perfect-information, turn-based games on a grid of any number
of dimensions. The aim is to reach games like Catan or Root (other boards, resources, dice,
cards, hidden hands, sides with their own rules) while staying the same kind of engine: games
described by a spec built from small rules that combine, not written as code.

This serves the hub's direction: people propose games, and the more a spec can say, the more
of those games the platform can host without anyone writing Rust.

## Goals

- A board is a graph of named positions. A tiled board (square, any number of dimensions, hex)
  is one kind of graph, and a free-form map (clearings joined by paths) is another.
- State can hold more than pieces on squares: places off the board (pools, hands, decks,
  discard piles, dice), several pieces in one place, in order where it matters.
- Turns can have stages that last while a condition holds.
- Dice and shuffles, kept fair and replayable.
- Hidden information: what each player may see.
- Rules and pieces that belong to one side only.
- Positions are addressed by name everywhere outside the engine, not by `x, y`.

## Non-goals

- Hosting Catan, Root or any other protected game. Their mechanics are the target; the games
  that prove each step are original.
- A scripting language or code escape hatch. Everything stays in the spec.
- Studying mathematically which games a set of rules can generate. It stays in mind (and is
  one more reason to keep the rule vocabulary small), but nothing here is built for it.
- Replacing the per-game switches (`hand_enabled`, `jump_enabled`, `mandatory_capture`, ...).
  They keep working; new capabilities are added as composable rules, not as more switches.

## Anti-goals

- Any change to how an existing game plays. Every game in `crates/engine/specs` behaves exactly
  as before at every step, and the engine's test suite passes unchanged at every step (tests
  may be added, and test helpers may change shape, but no expectation changes).
- Any stored match or stored spec version that stops loading.
- A second engine beside the first. There is one code path; grids do not keep a private one.

## Constraints

- **Stored data is immutable.** Spec versions are never edited (`crates/game/src/domain/spec.rs`)
  and matches store the engine's state as JSON (`matches.state`). The engine must keep reading
  every spec and every state it has ever written. The JSON of a grid game's spec and state does
  not change shape; new fields are optional.
- **The engine never panics outside tests** (`crates/engine/src/lib.rs` denies `unwrap`,
  `expect` and indexing): a panic freezes the board in the browser.
- **The engine also runs in the browser** (`crates/wasm`, the "Try it" board). Anything secret
  (step 7) must never reach it.
- **One PR per step**, each shippable on its own, in the order below.

## Approaches

How grids become a kind of graph, the base everything else rests on:

1. **A board interface with two implementations.** The engine asks the board for "the
   position one step this way from here", "every position", "the neighbours of here". A grid
   answers with arithmetic, a graph with tables. Cheap and fast, but every rule that looks
   at geometry (explosions, lines, adjacency) needs a grid answer and a graph answer, so the
   two drift apart.
2. **Grids compile into graphs (chosen).** At load time a grid spec is turned into a graph:
   one node per square, named `"x,y"` (or `"x,y,z"`), and one labelled edge for every step and
   offset the spec uses. The engine only ever walks graphs. One code path, exactly the model
   "a tiled board is a kind of graph", and a grid spec does not change. The cost is memory
   for the edge tables, which is small at board-game sizes (19×19 Go with 8 directions is
   under 3,000 edges).
3. **Keep coordinates and add a separate graph mode.** Least work now, but it is the second
   engine the anti-goals rule out.

Approach 2 was chosen by Claude after Frank handed over the how ("I like your approach").

## The route

Each step lists what it adds, how it is proven, and what it must not break. Steps 1 to 3 are
designed here in enough detail to plan. Steps 4 to 8 are sketched; each gets its own short
design when it is next, because the steps before it will have changed the code it builds on.

### Step 1: remove the remaining 2D limits

The design already allows any number of dimensions; a few places were written for two.

- The player's orientation (`Direction`, `crates/engine/src/shared/position.rs`) is a 2×2
  matrix. It becomes an N×N integer matrix, N being the board's number of dimensions, still
  required to have determinant ±1, and now with entries of -1, 0 or 1 only (every orientation a
  grid needs; larger entries could overflow when a step is turned). JSON for existing specs is unchanged (`[[1,0],[0,1]]`).
- Relative positions in conditions and side effects (`position`, `from`, `to`, `target`) are
  fixed pairs (`[i8; 2]`). They become vectors of the board's dimension.
- `Board::all_positions` returns nothing unless the board is 2D. It enumerates any dimension.
- `EXPLODE` and `ALLY_ADJACENT_COUNT` only look at the first two coordinates. Both look at
  every position within the radius (Chebyshev distance) across all axes.
- `Board::is_position_valid` accepts a position shorter than the board's dimensions. It
  requires the exact length.
- The spec checker rejects any step, offset or matrix whose size does not match the board, and
  any step or offset longer than 255 along an axis (it always lands off the board, and could
  overflow when added to a square).

Proof: a small original 3D game in the engine's tests (for example rooks and kings on a
4×4×4 board) plays, checks and mates correctly, and a 3D explosion clears all 26 neighbours.
3D games stay engine-only: the session and the board element still draw 2D.

### Step 2: the board is a graph inside the engine

Nothing a person or a spec sees changes. Inside the engine:

- A position is a node. Nodes have names; a grid's are its coordinates joined by commas
  (`"4,2"`), the same strings a saved state already uses for its keys, so stored states load
  unchanged. Internally a node may be an index for speed; that is left to the builder.
- Edges carry labels. Compiling a grid creates, for every distinct vector the spec uses (move
  steps, condition and side-effect offsets, after each player's orientation is applied), an
  edge from every square to the square that vector reaches, labelled by the vector.
- Each player has a label map: "my forward" means a different edge for White and Black. For a
  grid it is derived from the orientation matrix.
- A slide (`repeat`) follows the same label again. A line between two squares (the cannon's
  screen in `PATH_PIECE_COUNT`, castling's `PATH_NOT_ATTACKED`) is the walk along the unit
  direction, which grids compile as its own label: the vector divided down to steps of at most
  one per axis.
- Neighbourhoods are named edge sets. A grid provides `orthogonal` (one step along one axis:
  Go's liberties, area scoring) and `adjacent` (every square within one step on all axes:
  explosions, adjacent counts). An explosion of radius r is r rounds of `adjacent`.
- Repetition keys, attack maps and drops use node names in place of coordinates.

Proof: the full engine suite passes unchanged; a timing check shows move generation for
chess and 19×19 Go is not noticeably slower than before (the builder picks the measure and
records the before and after in the PR).

### Step 3: graph boards in specs, and positions by name

- A spec's board may be a graph: nodes with names and drawing coordinates, edges with labels,
  each player's label map, and the neighbourhoods the rules use. A square board keeps its
  current form, and a hex board gets a short form (`"shape": "hex"` and a size) that compiles
  to a graph like squares do. Exact JSON is for the step's plan.
- The session, the wasm wrapper and the game service's commands take positions by name. `x, y`
  stays accepted for 2D grids so old clients and stored commands keep working.
- The view sends each node's name and drawing coordinates, and its edges when the board is not
  a grid, so the front end can draw any board. The board element gains a drawing mode for
  graphs: hexes for hex boards, dots and lines for maps.

Proof: an original hex game (for example a connection game on an 11-hex board) is playable
end to end, in "Try it" and in a match.

### Step 4: places that hold many pieces (sketch)

- Places off the board: per player (hand, pool, tableau) and shared (supply, deck, discard,
  dice tray). They are nodes too, without edges unless the spec gives them some.
- A place can hold several pieces, either as counts of identical pieces (a pool of 3 wood) or
  as an ordered pile (a deck).
- Actions move pieces between places, so "buy a road" is a move from your pool to the supply
  plus a placement, and "draw" is top of deck to hand.
- Today's hand (`state.hand`) becomes a per-player place; old saved hands still load.
- Open for that step's design: counters (victory points, tracks) as piles of tokens or as
  plain numbers per player. Claude's lean: tokens for anything that moves between players,
  numbers for scores and tracks.

### Step 5: turns with stages (sketch)

The turn order accepts stages such as `{ "player": "RED", "while": <condition> }`: the player
keeps acting while the condition holds, and can end the stage when the spec allows it. Fixed
repetitions (`["BLACK", "BLACK", "WHITE"]`) keep working as they do.

### Step 6: dice and shuffles (sketch)

A random seed lives with the match on the server. Rolling puts random faces in a place;
shuffling reorders a pile. Each result is recorded in the history, so a match replays exactly
and nobody can predict the next roll from what they can see.

### Step 7: hidden information (sketch)

Each place says who can see its contents (its owner, everyone, nobody) and whether the
number of pieces shows. The server sends each seat only what that seat may see, and neither the
browser nor the wasm engine ever receives the rest. "Try it" (one person playing every side)
keeps working, without secrets.

### Step 8: sides with their own rules, and answers out of turn (sketch)

Moves can already be limited to some players (`MoveSpec::players`). This step extends that to
pieces, places, actions, stages and win conditions, so each side can play its own game. It also
adds decisions that other players must take outside their turn (everyone discards on a 7,
answering a trade offer): the turn waits until they answer.

## Proving the route

Each step that adds a capability ends with a small original game that needs it, in this order:
3D rooks (step 1), a hex connection game (step 3), a resource-and-building game (steps 4 and 5),
a dice race (step 6), a card-drafting game (step 7), an asymmetric game (step 8).

## Calls made after review

- **What "the test suite passes unchanged" allows.** Assertions never change. Call sites may be
  respelled mechanically when a public type changes (for example `vec![3, 0]` becoming a
  position name), so step 2 may change `Position` and `GameTransition` directly instead of
  hiding the graph behind a coordinate facade. Claude's call.
- **Board size limits.** Once every dimension is enumerated, a proposed spec could ask for a
  board too large to list. The checker refuses a board with no dimensions, a dimension of size
  0, more than 8 dimensions, or more than 10,000 positions (the largest board today is 19×19,
  361). Claude's call.

## Open before step 3

A reviewer reading this document as its builder raised these. None affects steps 1 and 2.
Each is settled with Frank before step 3 is planned, because step 3's spec format is stored
for good once a spec uses it.

1. On a graph board, do vector-based rules (offsets such as castling's `[3, 0]`, line
   conditions, explosion radius, `ALLY_ON_FILE`, orientation matrices) get a graph meaning
   (offsets as paths of labels), or does the checker refuse them there?
2. Is the hex short form a coordinate grid (axial vectors, so steps, offsets and matrices keep
   working) or a labelled graph whose moves name labels? And is a hex board a rhombus, a
   hexagon, or both?
3. Must the step 3 graph format already let pieces sit on paths and on tile corners (Catan's
   roads and settlements), or can step 4 extend it without a second format?
4. The proof game for step 3: a connection game needs a new win condition ("a chain joined
   through neighbourhood N touches zones A and B"); a hex game with existing rules (hex Go)
   needs none. Which?
5. Should the proof game be published in the catalogue (and in which family), or be playable
   in a match without being listed?

## Left to the builder

- How nodes are stored and looked up inside the engine (step 2).
- How the determinant of an N×N matrix is computed (step 1).
- Edge label syntax inside the engine, as long as a spec never has to write one for a grid.
- The timing measure in step 2.
