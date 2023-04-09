pub mod movements;

use movements::Movement;
use std::fmt;

use crate::board::position::Position;

#[derive(Clone, Eq, PartialEq)]
pub struct Piece {
    pub symbol: String,
    pub player: u8,
    pub movements: Vec<Movement>,
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

    /// With team
    pub fn with_team(piece: &Self, player: u8) -> Piece {
        Piece {
            player,
            symbol: piece.symbol.clone(),
            movements: piece.movements.clone(),
        }
    }
}

// Custom Debug trait implementation for visualization during development
impl<'a> fmt::Debug for Piece {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.symbol, self.player)
    }
}

pub struct PositionedPiece {
    pub position: Position,
    pub piece: Piece,
}

impl PositionedPiece {
    pub fn new(position: &Position, piece: &Piece) -> Self {
        PositionedPiece {
            position: *position,
            piece: piece.clone(),
        }
    }
}

