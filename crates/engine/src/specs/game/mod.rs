pub mod board;
pub mod condition;
pub mod draw_conditions;
pub mod liberty_capture;
pub mod piece;
pub mod player;
pub mod turns;
pub mod win_condition;

pub mod game;
pub mod parser;

pub use board::BoardSpec;
pub use draw_conditions::DrawConditionsSpec;
pub use game::{GameSpec, GameSpecError};
pub use liberty_capture::LibertyCaptureSpec;
pub use piece::{ActionSpec, ConditionSpec, MoveSpec, PieceSpec, ZoneRestrictionSpec};
pub use player::PlayerSpec;
pub use turns::TurnSpec;
pub use win_condition::WinConditionSpec;

//

#[cfg(test)]
mod tests;
