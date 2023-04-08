pub mod movements;

use std::fmt;

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Piece<'a> {
  pub symbol: &'a str,
  pub player: u8
}

impl<'a> Piece<'a> {
  /// Creates a new piece, assigning a symbol and a player
  pub fn new(symbol: &'a str, player: u8) -> Piece<'a> {
    Piece {
      symbol,
      player
    }
  }
}

// Custom Debug trait implementation for visualization during development
impl<'a> fmt::Debug for Piece<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      write!(f, "{}:{}", self.symbol, self.player)
  }
}