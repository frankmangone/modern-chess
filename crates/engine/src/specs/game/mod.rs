pub mod board;
pub mod condition;
pub mod draw_conditions;
pub mod hex_map;
pub mod liberty_capture;
pub mod map;
pub mod piece;
pub mod player;
pub mod test_block;
pub mod turns;
pub mod win_condition;

pub mod game;
pub mod parser;

pub use board::{BoardShape, BoardSpec, HexLayout};
pub use draw_conditions::DrawConditionsSpec;
pub use game::{GameSpec, GameSpecError};
pub use liberty_capture::LibertyCaptureSpec;
pub use map::{MapSource, MapSpec, PlaceKind, PlaceSpec};
pub use piece::{ActionSpec, ConditionSpec, MoveSpec, PieceSpec, StepSpec, ZoneRestrictionSpec};
pub use player::{PlayerSpec, PositionSpec};
pub use test_block::{ConditionEntry, TestSpec};
pub use turns::TurnSpec;
pub use win_condition::WinConditionSpec;

//

#[cfg(test)]
mod tests;
