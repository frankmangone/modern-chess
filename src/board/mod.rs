mod tests;

use crate::piece::{
    Piece,
    movements::{
        Action,
        Direction,
        Movement,
        ParsedMovement,
        Steps,
    }
};
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

pub struct Board {
    pub pieces: HashMap<Position, Piece>,
    pub dimensions: Dimensions,
}

impl Board {
    /// Creates a new empty board struct
    ///
    /// # Examples
    ///
    /// ```
    /// let mut board = Board::new(8, 8);
    /// ```
    pub fn new(rows: u8, cols: u8) -> Board {
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
    pub fn add_piece(&mut self, position: &Position, piece: &Piece) -> Result<(), BoardError> {        
        // Existing cannot place a piece in place of another (revisit this).
        if self.is_empty(position) {
            return Err(BoardError::PositionNotEmpty);
        }

        self.set_value(position, piece)
    }

    /// Find movements for a given selected position
    pub fn find_movements(&self, position: &Position) -> Result<Vec<Position>, BoardError> {
        let piece = self.get_value(&position)?;

        // TODO: check "ownership" depending on turn!

        let is_empty = piece == Option::None;

        if is_empty { return Ok(vec![]); }

        let movements = &piece.unwrap().movements;
        let mut calculated_movements: Vec<ParsedMovement> = Vec::new(); // TODO: THIS SHOULD HAVE POSITION AND ACTION!!

        for movement in movements {
            let action = &movement.action;

            match &movement.positions {
                [Direction::Ver(v_step), Direction::Hor(h_step)] |
                [Direction::Hor(h_step), Direction::Ver(v_step)] => {
                    self.add_movements(action, position, h_step, v_step, &mut calculated_movements)
                },
                [Direction::Ver(v_step), Direction::None] |
                [Direction::None, Direction::Ver(v_step)] => {
                    self.add_movements(action, position, &Steps::None, v_step, &mut calculated_movements)
                },
                [Direction::Hor(h_step), Direction::None] |
                [Direction::None, Direction::Hor(h_step)] => {
                    self.add_movements(action, position, h_step, &Steps::None, &mut calculated_movements);
                },
                // Player-based movements???
                _ => ()
            }
        }

        Ok(vec![])   
    }

    /// [Private] Adds movement based on the specifiec steps to take
    fn add_movements(
        &self,
        action: &Action,
        source: &Position,
        h_step: &Steps,
        v_step: &Steps,
        movements: &mut Vec<ParsedMovement>
    ) {
        match (h_step, v_step) {
            (Steps::None, Steps::PosValue(v_value)) => {
                // swap piece positions
                if self.is_empty(&Position(&source.0 + v_value, source.1)) {

                }
            },
            _ => ()
        }
    }

    /// Clears the board by removing all the stored pieces
    pub fn clear(&mut self) {
        self.pieces.clear();
    }

    /// [Private] Swaps the values of two positions
    fn swap(&self, pos_1: &Position, pos_2: &Position) -> Result<(), BoardError> {
        let val_1 = self.get_value(pos_1)?;
        let val_2 = self.get_value(pos_2)?;

        match (val_1, val_2) {
            (Some(val_1), Some(val_2)) => {
                self.set_value(pos_1, val_2)?;
                self.set_value(pos_2, val_1)?;
            },
            (Some(val_1), None) => {
                self.clear_value(pos_1)?;
                self.set_value(pos_2, val_1)?;
            },
            (None, Some(val_2)) => {
                self.set_value(pos_1, val_2)?;
                self.clear_value(pos_2)?;
            }
            _ => (), // No effect
        }

        Ok(())
    }

    /// [Private] Checks if a square is empty
    fn is_empty(&self, position: &Position) -> bool {
        self.pieces.contains_key(position)
    }

    /// [Private] Clears the value of a given position in the pieces hashmap
    fn clear_value(&mut self, position: &Position) -> Result<(), BoardError> {
        self.pieces.remove(position);
        Ok(())
    }

    /// [Private] Gets the value at a given position in the board
    /// The operation fails if the position is out of bounds
    fn get_value(&self, position: &Position) -> Result<Option<&Piece>, BoardError> {
        // Check if position is out of bounds
        if self.dimensions.0 <= position.0 || self.dimensions.1 <= position.1 {
            return Err(BoardError::OutOfBounds);
        }

        Ok(self.pieces.get(position))
    }

    /// [Private] Sets the value at a given position in the board
    /// The operation fails if the position is out of bounds
    fn set_value(&mut self, position: &Position, value: &Piece) -> Result<(), BoardError> {
        // Check if position is out of bounds
        if self.dimensions.0 <= position.0 || self.dimensions.1 <= position.1 {
            return Err(BoardError::OutOfBounds);
        }

        let piece_to_save = value.clone();

        self.pieces.insert(*position, piece_to_save);
        Ok(())
    }
}

