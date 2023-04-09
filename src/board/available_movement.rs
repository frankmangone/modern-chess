use std::fmt;
use crate::board::{position::Position, Board};
use crate::piece::PositionedPiece;
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
        source: &PositionedPiece,
    ) -> Option<Self> {
        let new_row = source.position.row() as i8 + v_change;
        let new_col = source.position.col() as i8 + h_change;

        // Avoid underflows and overflows
        if new_row < 0 || new_col < 0 || new_row > board.dimensions.rows() as i8 - 1 || new_col > board.dimensions.cols() as i8 - 1 {
          return None;
        }

        let new_position = Position::new(new_row as u8, new_col as u8);

        // Check if the cell is empty. If it's not, check if there's an enemy piece there (capture action).
        match board.get_value(&new_position).unwrap_or_default() {
            Some(piece)=> {
                // Square is not empty. Check team match.
                if piece.player == source.piece.player {
                    // Same team, disallow for now
                    None
                } else {
                    // TODO: Limit this to CAPTURE action
                    Some(AvailableMovement {
                        action: action.clone(),
                        position: new_position,
                    })
                }
            }
            None => {
                // Square is empty, allow for now.
                Some(AvailableMovement {
                    action: Action::Move,
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
