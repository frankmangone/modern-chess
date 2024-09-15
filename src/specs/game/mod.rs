pub mod board;
pub mod piece;

pub mod game;
pub mod parser;

pub use game::{GameSpec, GameSpecError};
pub use board::{BoardSpec, PlayerSpec, TurnSpec};
pub use piece::PieceSpec;

//

mod tests;