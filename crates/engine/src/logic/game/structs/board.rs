use std::collections::HashSet;
use crate::specs::BoardSpec;
use crate::shared::{Position, ExtendedPosition, into_position};

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
    /// Checks whether if a position is valid by examining out-of-bounds conditions
    /// and disabled positions.
    pub fn is_position_valid(&self, position: &ExtendedPosition) -> bool {
        for i in 0..position.len() {
            if position[i] < 0 || position[i] > self.dimensions[i] as i16 - 1i16 {
                // Value is outside of range.
                return false
            }

            if self.disabled_positions.contains(&into_position(position)) {
                // Value is in one of the known disabled positions.
                return false
            }
        }

        true
    }
}