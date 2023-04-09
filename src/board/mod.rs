pub mod available_movement;
pub mod position;
mod dimensions;
mod tests;

use crate::board::{available_movement::AvailableMovement, position::Position, dimensions::Dimensions};
use crate::piece::{
    movements::{Action, Direction, Movement, ParsedMovement, Steps},
    Piece,
};
use std::collections::HashMap;

pub enum BoardError {
    PositionNotEmpty,
    OutOfBounds,
}

pub struct Board {
    pub pieces: HashMap<Position, Piece>,
    pub dimensions: Dimensions,
    pub available_movements: Vec<AvailableMovement>,
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
            available_movements: Vec::new(),
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
    pub fn find_movements(&mut self, position: &Position) -> Result<(), BoardError> {
        let piece = self.get_value(&position)?;

        // TODO: check "ownership" depending on turn!

        let is_empty = piece == Option::None;

        if is_empty {
            return Ok(());
        }

        let movements = &piece.unwrap().movements.clone();
        self.available_movements = vec![];

        for movement in movements {
            let action = &movement.action;

            match &movement.positions {
                [Direction::Ver(v_step), Direction::Hor(h_step)]
                | [Direction::Hor(h_step), Direction::Ver(v_step)] => {
                    self.add_movements(action, position, v_step, h_step)
                }
                [Direction::Ver(v_step), Direction::None]
                | [Direction::None, Direction::Ver(v_step)] => self.add_movements(
                    action,
                    position,
                    v_step,
                    &Steps::None,
                ),
                [Direction::Hor(h_step), Direction::None]
                | [Direction::None, Direction::Hor(h_step)] => {
                    self.add_movements(
                        action,
                        position,
                        &Steps::None,
                        h_step,
                    );
                }
                // Player-based movements???
                _ => (),
            }
        }

        Ok(())
    }

    /// [Private] Adds movement based on the specifiec steps to take
    fn add_movements(
        &mut self,
        action: &Action,
        source: &Position,
        h_step: &Steps,
        v_step: &Steps,
    ) {
        // Default "None" steps to 0 for easier handling
        let v_step = match v_step {
            Steps::None => &Steps::Value(0),
            other => other,
        };

        let h_step = match h_step {
            Steps::None => &Steps::Value(0),
            other => other,
        };

        let mut new_moves: Vec<AvailableMovement> = match (h_step, v_step) {
            (Steps::Value(h_value), Steps::Value(v_value)) => {
                let new_move = 
                    AvailableMovement::new(
                        &self,
                        action,
                        *v_value,
                        *h_value,
                        source,
                    );

                match new_move {
                    Option::Some(value) => vec![value],
                    Option::None => vec![],
                }
            },
            (Steps::Every(h_value), Steps::Value(v_value)) => {
                let mut new_moves: Vec<AvailableMovement> = vec![];
                let mut cummulative_h_step = 0;

                loop {
                    cummulative_h_step += h_value;
                    let new_move = AvailableMovement::new(
                        &self,
                        action,
                        *v_value,
                        cummulative_h_step,
                        source,
                    );

                    match new_move {
                        Option::Some(value) => {
                            new_moves.push(value);
                        },
                        Option::None => {
                            break;
                        },
                    }
                }

                new_moves
            },
            (Steps::Value(h_value), Steps::Every(v_value)) => {
                let mut new_moves: Vec<AvailableMovement> = vec![];
                let mut cummulative_v_step = 0;

                loop {
                    cummulative_v_step += v_value;
                    let new_move = AvailableMovement::new(
                        &self,
                        action,
                        cummulative_v_step,
                        *h_value,
                        source,
                    );

                    match new_move {
                        Option::Some(value) => {
                            new_moves.push(value);
                        },
                        Option::None => {
                            break;
                        },
                    }
                }

                new_moves
            },
            (Steps::Every(h_value), Steps::Every(v_value)) => {
                let mut new_moves: Vec<AvailableMovement> = vec![];
                let mut cummulative_h_step = 0;
                let mut cummulative_v_step = 0;

                loop {
                    cummulative_h_step += h_value;
                    cummulative_v_step += v_value;
                    let new_move = AvailableMovement::new(
                        &self,
                        action,
                        cummulative_v_step,
                        cummulative_h_step,
                        source,
                    );

                    match new_move {
                        Option::Some(value) => {
                            new_moves.push(value);
                        },
                        Option::None => {
                            break;
                        },
                    }
                }

                new_moves
            },
            _ => vec![],
        };

        self.available_movements.append(&mut new_moves);
    }

    /// Clears the board by removing all the stored pieces
    pub fn clear(&mut self) {
        self.pieces.clear();
    }

    /// [Private] Swaps the values of two positions
    // fn swap(&self, pos_1: &Position, pos_2: &Position) -> Result<(), BoardError> {
    //     let val_1 = self.get_value(pos_1)?;
    //     let val_2 = self.get_value(pos_2)?;

    //     match (val_1, val_2) {
    //         (Some(val_1), Some(val_2)) => {
    //             self.set_value(pos_1, val_2)?;
    //             self.set_value(pos_2, val_1)?;
    //         }
    //         (Some(val_1), None) => {
    //             self.clear_value(pos_1)?;
    //             self.set_value(pos_2, val_1)?;
    //         }
    //         (None, Some(val_2)) => {
    //             self.set_value(pos_1, val_2)?;
    //             self.clear_value(pos_2)?;
    //         }
    //         _ => (), // No effect
    //     }

    //     Ok(())
    // }

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
