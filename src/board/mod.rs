mod tests;

use crate::piece::Piece;
use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position(pub u8, pub u8); // Positions are zero-indexed
pub struct Dimensions(pub u8, pub u8);

// Custom Debug trait implementation for visualization during development
impl fmt::Debug for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

pub enum BoardError {
    PositionNotEmpty,
    OutOfBounds
}

pub struct Board<'a> {
    pub pieces: HashMap<Position, Piece<'a>>,
    pub dimensions: Dimensions,
}

impl<'a> Board<'a> {
    /// Creates a new empty board struct
    ///
    /// # Examples
    ///
    /// ```
    /// let mut board = Board::new(8, 8);
    /// ```
    pub fn new(rows: u8, cols: u8) -> Board<'a> {
        Board {
            pieces: HashMap::new(),
            dimensions: Dimensions(rows, cols),
        }
    }

    /// Adds a piece to the board
    /// The operation fails if the position is already occupied
    ///
    /// # Examples
    ///
    /// ```
    /// let mut board = Board::new(8, 8);
    /// board.add_piece(&Position(1,1), piece_1); // Returns Ok(())
    /// board.add_piece(&Position(1,1), piece_2); // Returns Err(BoardError::PositionNotEmpty)
    /// board.add_piece(&Position(8,1), piece_2); // Returns Err(BoardError::OutOfBounds)
    /// ```
    pub fn add_piece(&mut self, position: &Position, piece: &'a Piece) -> Result<(), BoardError> {        
        // Existing cannot place a piece in place of another (revisit this).
        if self.pieces.contains_key(position) {
            return Err(BoardError::PositionNotEmpty);
        }

        self.set_value(position, piece)
    }

    /// Clears the board by removing all the stored pieces
    pub fn clear(&mut self) {
        self.pieces.clear();
    }

    /// [Private] Sets the value at a given position in the board
    /// The operation fails if the position is out of bounds
    fn set_value(&mut self, position: &Position, value: &'a Piece) -> Result<(), BoardError> {
        // Check if position is out of bounds
        if self.dimensions.0 <= position.0 || self.dimensions.1 <= position.1 {
            return Err(BoardError::OutOfBounds);
        }

        let piece_to_save = value.clone();

        self.pieces.insert(*position, piece_to_save);
        Ok(())
    }
}

