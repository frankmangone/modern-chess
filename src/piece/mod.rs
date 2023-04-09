pub mod movements;

use std::fmt;
use movements::Movement;

#[derive(Clone, Eq, PartialEq)]
pub struct Piece {
  pub symbol: String,
  pub player: u8,
  pub movements: Vec<Movement>
}

impl Piece {
  /// Creates a new piece, assigning a symbol and a player
  pub fn new(symbol: String, player: u8, movements: Vec<Movement>) -> Piece {
    Piece {
      symbol,
      player,
      movements,
    }
  }
}

// Custom Debug trait implementation for visualization during development
impl<'a> fmt::Debug for Piece {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      write!(f, "{}:{}", self.symbol, self.player)
  }
}