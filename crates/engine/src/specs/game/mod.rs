pub mod action;
pub mod board;
pub mod draw_conditions;
pub mod end_check;
pub mod hex_map;
pub mod map;
pub mod old_forms;
pub mod piece;
pub mod player;
pub mod rule_steps;
pub mod test_block;
pub mod turns;
pub mod win_condition;
pub mod zone;

pub mod game;
pub mod parser;

pub use action::{ActionSpec, ChoiceSpec, CodeSpec, DoSpec, EffectSpec, IfSpec};
pub use board::{BoardShape, BoardSpec, HexLayout};
pub use draw_conditions::DrawConditionsSpec;
pub use end_check::{EndCheckSpec, OutcomeSpec, ScoreTermSpec, Viewpoint};
pub use game::{GameSpec, GameSpecError};
pub use map::{MapSource, MapSpec, PlaceKind, PlaceSpec};
pub use piece::{MoveSpec, PieceSpec, StepSpec};
pub use player::{PlayerSpec, PositionSpec};
pub use rule_steps::EveryActionSpec;
pub use test_block::TestSpec;
pub use turns::TurnSpec;
pub use win_condition::WinConditionSpec;

//

#[cfg(test)]
mod tests;
