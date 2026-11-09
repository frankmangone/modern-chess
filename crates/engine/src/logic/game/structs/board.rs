use crate::shared::{into_position, ExtendedPosition, Position};
use crate::specs::BoardSpec;
use std::collections::HashSet;

/// A `Board` is a representation of everything board-related. Of course,
/// boards contain pieces, and have a shape that establishes which positions
/// are viable.
#[derive(Clone, Debug)]
pub struct Board {
    // Board shape specifications
    pub dimensions: Vec<u8>,
    pub disabled_positions: HashSet<Position>,
}

// ---------------------------------------------------------------------
// Associated fns to parse spec
// ---------------------------------------------------------------------
impl Board {
    pub fn from_spec(board_spec: BoardSpec) -> Board {
        Board {
            dimensions: board_spec.dimensions,
            disabled_positions: board_spec.disabled_positions,
        }
    }
}

// ---------------------------------------------------------------------
// Logic-related associated fns
// ---------------------------------------------------------------------
impl Board {
    /// Returns every valid (non-disabled, in-bounds) position on the board.
    /// Only works for 2-D boards; returns an empty Vec for other dimensionalities.
    pub fn all_positions(&self) -> Vec<Position> {
        let [cols, rows] = self.dimensions[..] else {
            return vec![];
        };
        let mut result = Vec::with_capacity((cols as usize) * (rows as usize));
        for col in 0..cols {
            for row in 0..rows {
                let pos: Position = vec![col, row];
                if !self.disabled_positions.contains(&pos) {
                    result.push(pos);
                }
            }
        }
        result
    }

    /// Checks whether if a position is valid by examining out-of-bounds conditions
    /// and disabled positions.
    pub fn is_position_valid(&self, position: &ExtendedPosition) -> bool {
        if position.len() > self.dimensions.len() {
            return false;
        }
        for (value, dimension) in position.iter().zip(&self.dimensions) {
            if *value < 0 || *value > *dimension as i16 - 1i16 {
                // Value is outside of range.
                return false;
            }

            if self.disabled_positions.contains(&into_position(position)) {
                // Value is in one of the known disabled positions.
                return false;
            }
        }

        true
    }
}
