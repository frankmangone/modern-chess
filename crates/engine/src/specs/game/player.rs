use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::shared::{determinant, Direction, Position};
use crate::specs::{GameSpecError, Validate};

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
    pub positions: Vec<Position>,
}

#[cfg(test)]
impl PlayerSpec {
    /// Creates an empty player, with just a name. Used only for tests.
    pub fn from_name(name: &str) -> PlayerSpec {
        PlayerSpec {
            name: name.to_string(),
            direction: vec![vec![1i16, 0i16], vec![0i16, 1i16]], // identity — moves "up"
            starting_positions: vec![],
        }
    }
}

impl Validate for PlayerSpec {
    // Known pieces HashSet
    type Arg1 = HashSet<String>;

    // Board dimensions
    type Arg2 = BoardSpec;

    /// Validates the player spec contents
    fn validate(
        &self,
        piece_names: &HashSet<String>,
        board: &BoardSpec,
    ) -> Result<(), GameSpecError> {
        // The direction matrix must be N×N for a board of N dimensions...
        let expected = board.dimensions.len();
        let found = if self.direction.len() != expected {
            Some(self.direction.len())
        } else {
            self.direction
                .iter()
                .map(Vec::len)
                .find(|&length| length != expected)
        };
        if let Some(found) = found {
            return Err(GameSpecError::MismatchedDimensions {
                what: format!("direction of player {}", self.name),
                expected,
                found,
            });
        }

        // ...with determinant ±1 (rotation or rotation+reflection).
        if !matches!(determinant(&self.direction), Some(1) | Some(-1)) {
            return Err(GameSpecError::InvalidDirectionMatrix(
                self.direction.clone(),
            ));
        }

        // Check starting positions.
        for positions_spec in &self.starting_positions {
            // Check that the pieces in the positions are valid.
            if !piece_names.contains(&positions_spec.piece) {
                return Err(GameSpecError::UnknownPieceInStartingPosition(
                    positions_spec.piece.clone(),
                ));
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
