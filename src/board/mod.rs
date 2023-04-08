use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position(pub u8, pub u8); // Positions are zero-indexed
pub struct Dimensions(pub u8, pub u8);

impl fmt::Debug for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

pub struct Board {
    pub pieces: HashMap<Position, String>,
    pub dimensions: Dimensions,
}

pub enum BoardError {
    TileNotEmpty,
    OutOfBounds
}

impl Board {
    //
    // Creates a new board struct
    pub fn new(rows: u8, cols: u8) -> Board {
        Board {
            pieces: HashMap::new(),
            dimensions: Dimensions(rows, cols),
        }
    }

    //
    // Adds a piece to an existing board
    // TODO: Consider using some sort of encoding for the position
    pub fn add_piece(&mut self, position: &Position, piece: &str) -> Result<(), BoardError> {        
        // Existing cannot place a piece in place of another (revisit this).
        if self.pieces.contains_key(position) {
            return Err(BoardError::TileNotEmpty);
        }

        // Check if position is out of bounds
        if self.dimensions.0 <= position.0 || self.dimensions.1 <= position.1 {
            return Err(BoardError::OutOfBounds);
        }

        self.pieces.insert(*position, String::from(piece));
        Ok(())
    }
}

