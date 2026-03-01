use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::specs::{Validate, GameSpecError};
use crate::shared::{Position, Direction};

use super::BoardSpec;

/// Player specs, determining a name, and important information to know their initial state
/// and transitions in the game.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlayerSpec {
    /// Player name, which doubles up as a unique identifier.
    pub name: String,

    /// A 2×2 rotation matrix representing this player's orientation on the board.
    /// All canonical move steps in piece specs are transformed by this matrix at
    /// blueprint-build time, so a single piece spec works correctly for every player.
    pub direction: Direction,

    /// Starting positions for all pieces for this player.
    pub starting_positions: Vec<PiecePositionSpec>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PiecePositionSpec {
    /// Piece name, which identifies a Piece, whose spec should be loaded.
    pub piece: String,

    /// Positions where the specified piece should be.
    /// A Position is just a Vec<u8>,
    pub positions: Vec<Position>
}

#[cfg(test)]
impl PlayerSpec {
    /// Creates an empty player, with just a name. Used only for tests.
    pub fn from_name(name: &str) -> PlayerSpec {
        PlayerSpec {
            name: name.to_string(),
            direction: [[1i16, 0i16], [0i16, 1i16]], // identity — moves "up"
            starting_positions: vec![]
        }
    }
}

impl Validate for PlayerSpec {
    // Known pieces HashSet
    type Arg1 = HashSet<String>;

    // Board dimensions
    type Arg2 = BoardSpec;

    /// Validates the player spec contents
    fn validate(&self, piece_names: &HashSet<String>, board: &BoardSpec) -> Result<(), GameSpecError> {
        // A valid direction matrix must have determinant ±1 (rotation or rotation+reflection).
        // det([[a, b], [c, d]]) = a*d - b*c
        let [[a, b], [c, d]] = self.direction;
        let det = a * d - b * c;
        if det != 1 && det != -1 {
            return Err(GameSpecError::InvalidDirectionMatrix(self.direction));
        }

        // Check starting positions.
        for positions_spec in &self.starting_positions {
            // Check that the pieces in the positions are valid.
            if !piece_names.contains(&positions_spec.piece) {
                return Err(GameSpecError::UnknownPieceInStartingPosition(positions_spec.piece.clone()));
            }

            // Check that the positions themselves are valid on the board.
            for position in &positions_spec.positions {
                // Check for correct dimensions.
                if position.len() != board.dimensions.len() {
                    return Err(GameSpecError::InvalidPositionDimensions(position.clone()));
                }

                // Check that position is not disabled.
                if board.disabled_positions.contains(position) {
                    return Err(GameSpecError::PositionDisabled(position.clone()));
                }
            }
        }

        Ok(())
    }
}
