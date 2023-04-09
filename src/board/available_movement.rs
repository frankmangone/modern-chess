use std::fmt;
use crate::board::{position::Position, Board};
use crate::piece::movements::Action;

pub struct AvailableMovement {
    pub position: Position,
    pub action: Action,
}

impl AvailableMovement {
    // Creates an available movement by checking conditions
    // that should be met before actually adding the movement
    pub fn new(
        board: &Board,
        action: &Action,
        v_change: i8,
        h_change: i8,
        source: &Position,
    ) -> Option<Self> {
        let new_row = source.row() as i8 + v_change;
        let new_col = source.col() as i8 + h_change;

        // Avoid underflows and overflows
        if new_row < 0 || new_col < 0 || new_row > board.dimensions.rows() as i8 - 1 || new_col > board.dimensions.cols() as i8 - 1 {
          return None;
        }

        let new_position = Position::new(new_row as u8, new_col as u8);

        // TODO: Fix this!
        // Check if the cell is empty. In fact, this should check if there's an enemy here! (On capture)
        match board.get_value(&new_position).unwrap_or_default() {
            Some(_) => {
                // TODO: This is what should be fixed
                // Square is not empty, disallow for not
                None
            }
            None => {
                // Square is empty, allow for now.
                Some(AvailableMovement {
                    action: action.clone(),
                    position: new_position,
                })
            },
        }

    }
}

// Custom Debug trait implementation for visualization during development
impl fmt::Debug for AvailableMovement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({:?}, {}, {})", self.action, self.position.0, self.position.1)
    }
}
