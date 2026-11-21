# Tidying the engine before step 3

## Why

Step 3 (hex boards) adds a win condition and a neighbourhood to the engine. Three things in
today's code make that harder than it needs to be, and Frank asked for them to be cleaned up
first, in their own pull request, before step 3 starts (2026-11-21).

1. Rule codes are kept as text and turned into an enum each time a rule is checked, inside the
   move generator.
2. `Game` is about twenty loose fields, most of which never change after the game is built. Atomic
   chess copies the whole `Game`, blueprints and all, to try one explosion.
3. The modules of `logic/game` are grouped by kind of Rust item (`structs/`, `enums/`), not by
   job; glob re-exports reach one type by several paths; two different things are called
   "conditions"; `move_blueprint.rs` (683 lines) mixes building with move generation; and
   `pub mod tests` in `lib.rs` is not behind `#[cfg(test)]`.

## Goals

- Rule codes are parsed into enums once, when the game is built.
- What never changes after build lives in one `Rules` value behind an `Arc`, so copying a game
  copies its position, not its rules.
- `logic` is grouped by job, with explicit re-exports.

## Non-goals and anti-goals

- No behaviour change. Every test assertion stays as it is; call sites may be respelled
  mechanically (`game.hand_enabled` becoming `game.rules.hand_enabled`), as in step 2.
- The stored shape does not change: `GameState`, `Effect` and `MoveRecord` keep their fields
  and their text actions (`"CAPTURE"`, `"PASS"`), since saved matches hold them.
- Specs do not change, and unknown codes stay ignored as today (making them build errors is a
  behaviour change, left for another day).
- No performance work beyond what the change gives for free.

## Approaches

- **Enums parsed at build (chosen).** Each code becomes an enum when the blueprint or game is
  built; every `match` is exhaustive and the blueprints stay `Clone` and `Debug`.
- Closures (`Box<dyn Fn>`) built at load time: as fast, but blueprints lose `Debug`, `Clone`
  needs `Arc`, and a reader can no longer see which rule a condition is.
- Leave the codes as text: no change; the cost is small, but every new rule adds another
  string match.

## The design

### Rule codes as enums

- `Condition.code: String` becomes `Condition.kind: ConditionKind`, with one variant per built-in
  code and `Named(String)` for a condition declared in the spec's `conditions` list (today's
  `Unknown` arm).
- `SideEffectBlueprint.action: String` becomes `SideEffectKind`, keeping `Unknown` (ignored).
- Win conditions are stored on the rules as a parsed `WinCondition` whose `kind` is
  `WinConditionKind`, keeping `Unknown` (ignored).
- The drop restriction code and a declared condition's type (`POSITION`, `STATE`) become enums
  the same way.
- `Effect.action` and `MoveRecord.action` stay text (stored data).

### Rules behind an `Arc`

`Game` becomes `{ rules: Arc<Rules>, board: Board, turn_order: Vec<String>, state: GameState }`.
`Rules` holds every field that is set at build and never changed: name, conditions, players,
blueprints, leader, the draw settings, the rule flags, piece values, demotions, win
conditions, players with a configured leader, liberty capture and unlimited supply.
`turn_order` stays on `Game` because eliminating a player (four-player chess) shrinks it. Tests
that change a rule use `Arc::make_mut(&mut game.rules)`. Claude's call.

### Modules

```
logic/
  blueprint/
    move_blueprint/   mod.rs (the types), build.rs (from_spec), generate.rs (moves, threats)
    piece_blueprint.rs, conditions/, side_effects/   (unchanged)
  game/
    mod.rs            Game and Rules (was main.rs)
    board/            mod.rs (Board), graph.rs
    state/            mod.rs (GameState), piece.rs, move_record.rs
    phase.rs, transition.rs, error.rs                 (were enums/)
    cell_conditions.rs  ConditionDef and the checks on a cell (were structs/condition.rs
                        and conditions.rs)
    handlers/, attack, board_changes, draw, io, legal, liberties, scoring, transitions, win
```

Modules are `pub(crate)` unless another crate uses them; `logic` re-exports by name what the
other crates use (`Game`, `GamePhase`, `GameTransition` and the types in `Game`'s public
fields and methods). `lib.rs` has `#[cfg(test)] mod tests`. Files move with `git mv`. Exact
names are Claude's call.

## Left to the builder

- Order of the three changes and how they are split into commits.
- Which enum each small code becomes, and where it is defined.

## Noticed, not in scope

- `turn_order` is not part of the saved state, so a four-player match restored after an
  elimination may get its eliminated player back. To check separately.
