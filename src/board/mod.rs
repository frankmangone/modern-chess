pub mod available_movement;
pub mod position;
pub mod presets;
mod dimensions;
mod tests;

use std::fmt;
use crate::board::{available_movement::AvailableMovement, position::Position, dimensions::Dimensions};
use crate::piece::{
    movements::{Action, Direction, Movement, ParsedMovement, Steps},
    Piece,
    PositionedPiece,
};
use std::collections::HashMap;

pub enum BoardError {
    PositionNotEmpty,
    OutOfBounds,
    UnavailableMove,
    UnknownMove,
}

pub struct Board {
    pub pieces: HashMap<Position, Piece>,
    pub dimensions: Dimensions,
    pub available_movements: HashMap<Position, AvailableMovement>,
    pub movement_source: Option<PositionedPiece>
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
            available_movements: HashMap::new(),
            movement_source: Option::None,
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

    /// Performs a movement in the `available_movements` list of the board.
    pub fn perform_movement(&mut self, position: &Position) -> Result<(), BoardError> {
        // TODO: Is this even needed? There are borrowing problems if not done this way I think.
        let data = {
            let aux = self.available_movements.get(position);
            match aux {
                Some(value) => Some(value.action.clone()),
                None => None,
            }
        };

        // FIXME: Need to know the source of the move!

        match data {
            None => Err(BoardError::UnavailableMove),
            Some(action) => {
                // TODO: For specific actions, it may be necessary to update this logic
                let (source_position, source_piece) = {
                    let source = self.movement_source.as_ref().unwrap();
                    (source.position.clone(), source.piece.clone())
                };

                let return_value = match action {
                    Action::Move => {
                        self.swap(position, &source_position).ok();
                        Ok(())
                    },
                    Action::Capture => {
                        self.clear_value(&source_position).ok();
                        self.set_value(&position, &source_piece).ok();
                        Ok(())
                    }
                    _ => Err(BoardError::UnknownMove)
                };

                self.clear_available_movements();

                return_value
            }
        }
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
        let source = PositionedPiece::new(position, &piece.unwrap());
        self.clear_available_movements();
        self.set_movement_source(&source);
        
        for movement in movements {
            let action = &movement.action;

            match &movement.positions {
                [Direction::Ver(v_step), Direction::Hor(h_step)]
                | [Direction::Hor(h_step), Direction::Ver(v_step)] => {
                    self.add_movements(action, &source, v_step, h_step)
                }
                [Direction::Ver(v_step), Direction::None]
                | [Direction::None, Direction::Ver(v_step)] => self.add_movements(
                    action,
                    &source,
                    v_step,
                    &Steps::None,
                ),
                [Direction::Hor(h_step), Direction::None]
                | [Direction::None, Direction::Hor(h_step)] => {
                    self.add_movements(
                        action,
                        &source,
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

    /// Clears the board by removing all the stored pieces
    pub fn clear(&mut self) {
        self.pieces.clear();
    }

    /// [Private] Adds movement based on the specific steps to take
    fn add_movements(
        &mut self,
        action: &Action,
        source: &PositionedPiece,
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

        let mut stop: bool = false;

        match (h_step, v_step) {
            (Steps::Value(h_value), Steps::Value(v_value)) => {
                let new_move = 
                    AvailableMovement::new(
                        self,
                        action,
                        *v_value,
                        *h_value,
                        source,
                    );

                match new_move {
                    Option::Some(value) => {
                        self.add_available_movement(value);
                    },
                    Option::None => (),
                }
            },
            (Steps::Every(h_value), Steps::Value(v_value)) => {
                let mut cummulative_h_step = 0;

                while !stop {
                    cummulative_h_step += h_value;
                    let new_move = AvailableMovement::new(
                        self,
                        action,
                        *v_value,
                        cummulative_h_step,
                        source,
                    );

                    self.eval_movement(new_move, &mut stop)
                }
            },
            (Steps::Value(h_value), Steps::Every(v_value)) => {
                let mut cummulative_v_step = 0;

                while !stop {
                    cummulative_v_step += v_value;
                    let new_move = AvailableMovement::new(
                        self,
                        action,
                        cummulative_v_step,
                        *h_value,
                        source,
                    );

                    self.eval_movement(new_move, &mut stop)
                }
            },
            (Steps::Every(h_value), Steps::Every(v_value)) => {
                let mut cummulative_h_step = 0;
                let mut cummulative_v_step = 0;
                
                while !stop {
                    cummulative_h_step += h_value;
                    cummulative_v_step += v_value;
                    let new_move = AvailableMovement::new(
                        self,
                        action,
                        cummulative_v_step,
                        cummulative_h_step,
                        source,
                    );

                    self.eval_movement(new_move, &mut stop)
                }
            },
            _ => (),
        };
    }

    /// [Private] Evaluates whether if a movement can be added, and checks a 
    /// stop condition
    fn eval_movement(&mut self, new_move: Option<AvailableMovement>, stop: &mut bool) {
        match new_move {
            Option::Some(value) => {
                let action = value.action.clone();

                self.add_available_movement(value);

                match action {
                    Action::Capture => *stop = true,
                    _ => ()
                }
            },
            Option::None => {
                *stop = true;
            },
        }
    }

    /// [Private] Sets the source of the movements in `available_movements`
    fn set_movement_source(&mut self, positioned_piece: &PositionedPiece) {
        self.movement_source = Option::Some(positioned_piece.clone());
    }

    /// [Private] Adds a new available movement to the available moves hashmap
    fn add_available_movement(&mut self, movement: AvailableMovement) {
        self.available_movements.insert(
            Position::new(movement.position.row(), movement.position.col()),
            movement,
        );
    }

    /// [Private] Clears all available movements
    fn clear_available_movements(&mut self) {
        self.available_movements = HashMap::new();
    }

    /// [Private] Swaps the values of two positions
    fn swap(&mut self, pos_1: &Position, pos_2: &Position) -> Result<(), BoardError> {
        // INFO: This is done this way so we can get rid of unnecessary borrows.
        let val_1: Option<Piece> = {
            let aux = self.get_value(pos_1)?;
            match aux {
                Some(value) => Some(value.clone()),
                None => None,
            }
        };

        let val_2: Option<Piece> = {
            let aux = self.get_value(pos_2)?;
            match aux {
                Some(value) => Some(value.clone()),
                None => None,
            }
        };

        match (val_1, val_2) {
            (Some(val_1), Some(val_2)) => {
                self.set_value(pos_1, &val_2)?;
                self.set_value(pos_2, &val_1)?;
            }
            (Some(val_1), None) => {
                self.clear_value(pos_1)?;
                self.set_value(pos_2, &val_1)?;
            }
            (None, Some(val_2)) => {
                self.set_value(pos_1, &val_2)?;
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

// Custom Debug trait implementation for visualization during development
impl fmt::Debug for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut str = String::new();
        str.push_str("\n");

        for col in 0..self.dimensions.cols() {
            for row in 0..self.dimensions.rows() {
                // TODO: This could error out!!
                let piece = self.get_value(&Position::new(row, col));
                let available_movement = self.available_movements.get(&Position::new(row, col));

                match available_movement {
                    Some(_) => str.push_str("[o0o]"),
                    None => {
                        match piece {
                            Ok(piece) => {
                                match piece {
                                    Some(value) => str.push_str(&format!("[{}]", &value.symbol[..3])),
                                    None => str.push_str("[___]")
                                }
                            }
                            Err(_) => str.push_str("[---]"),
                        }
                    },
                }
            }
            str.push_str("\n");
        }

        write!(f, "{}", str)
    }
}
