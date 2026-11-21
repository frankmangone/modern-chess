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
    /// Returns every valid (non-disabled, in-bounds) position on the board, whatever its number
    /// of dimensions, in lexicographic order (first coordinate outermost).
    pub fn all_positions(&self) -> Vec<Position> {
        let ranges: Vec<(u8, u8)> = self
            .dimensions
            .iter()
            .map(|&size| (0, size.saturating_sub(1)))
            .collect();
        if self.dimensions.contains(&0) {
            return vec![];
        }
        self.positions_in_ranges(&ranges)
    }

    /// Returns every valid position within `radius` of `center` along every axis (Chebyshev
    /// distance), `center` included, in the same order as `all_positions`. Empty when `center`
    /// does not have one coordinate per dimension of the board.
    pub fn positions_within(&self, center: &Position, radius: u8) -> Vec<Position> {
        if center.len() != self.dimensions.len() {
            return vec![];
        }
        let mut ranges = Vec::with_capacity(center.len());
        for (&coordinate, &size) in center.iter().zip(&self.dimensions) {
            if coordinate >= size {
                return vec![];
            }
            let last = size - 1;
            ranges.push((
                coordinate.saturating_sub(radius),
                coordinate.saturating_add(radius).min(last),
            ));
        }
        self.positions_in_ranges(&ranges)
    }

    /// Every enabled position whose coordinates each fall within the matching inclusive range,
    /// counting like an odometer with the last coordinate turning fastest.
    fn positions_in_ranges(&self, ranges: &[(u8, u8)]) -> Vec<Position> {
        let mut result = Vec::new();
        let mut current: Position = ranges.iter().map(|&(low, _)| low).collect();
        if current.is_empty() {
            return result;
        }
        loop {
            if !self.disabled_positions.contains(&current) {
                result.push(current.clone());
            }
            // Advance the odometer; stop once every coordinate has wrapped.
            let mut axis = current.len();
            loop {
                if axis == 0 {
                    return result;
                }
                axis -= 1;
                let (Some(value), Some(&(low, high))) = (current.get_mut(axis), ranges.get(axis))
                else {
                    return result;
                };
                if *value < high {
                    *value += 1;
                    break;
                }
                *value = low;
            }
        }
    }

    /// Checks whether if a position is valid by examining out-of-bounds conditions
    /// and disabled positions.
    pub fn is_position_valid(&self, position: &ExtendedPosition) -> bool {
        if position.len() != self.dimensions.len() {
            return false;
        }
        for (value, dimension) in position.iter().zip(&self.dimensions) {
            if *value < 0 || *value > *dimension as i16 - 1i16 {
                // Value is outside of range.
                return false;
            }
        }

        // In bounds; valid unless it is one of the known disabled positions.
        !self.disabled_positions.contains(&into_position(position))
    }
}
