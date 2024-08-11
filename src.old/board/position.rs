use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position(pub u8, pub u8); // Positions are zero-indexed

impl Position {
    /// Creates a position from row/column data
    pub fn new(row: u8, column: u8) -> Self {
        Position(row, column)
    }

    /// Gets the row of a position
    pub fn row(&self) -> u8 {
        self.0
    }

    /// Gets the column of a position
    pub fn col(&self) -> u8 {
        self.1
    }
}

// Custom Debug trait implementation for visualization during development
impl fmt::Debug for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}
