pub mod action;
pub mod bag;
pub mod board;
pub mod end_check;
pub mod hex_map;
pub mod map;
pub mod piece;
pub mod player;
pub mod rule_steps;
pub mod test_block;
pub mod turns;
pub mod zone;

pub mod game;
pub mod parser;

pub use action::{ActionSpec, ChoiceSpec, CodeSpec, CountSpec, DoSpec, EffectSpec, IfSpec};
pub use bag::{BagFor, BagRef, BagSpec};
pub use board::{BoardShape, BoardSpec, HexLayout};
pub use end_check::{EndCheckSpec, OutcomeSpec, ScoreTermSpec, Viewpoint};
pub use game::{GameSpec, GameSpecError};
pub use map::{MapSource, MapSpec, PlaceKind, PlaceSpec};
pub use piece::{MoveSpec, PieceSpec, StepSpec};
pub use player::{PlayerSpec, PositionSpec};
pub use rule_steps::EveryActionSpec;
pub use test_block::TestSpec;
pub use turns::{StageSpec, TurnSpec};

//

#[cfg(test)]
mod tests;
