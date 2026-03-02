pub mod condition;
pub mod board;
pub mod player;
pub mod piece;
pub mod turns;
pub mod draw_conditions;

pub mod game;
pub mod parser;

pub use game::{GameSpec, GameSpecError};
pub use board::BoardSpec;
pub use player::PlayerSpec;
pub use turns::TurnSpec;
pub use piece::{ActionSpec, PieceSpec, MoveSpec};
pub use draw_conditions::DrawConditionsSpec;

//

#[cfg(test)]
mod tests;
