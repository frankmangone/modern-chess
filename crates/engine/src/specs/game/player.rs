use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::shared::{determinant, Coordinates, Direction, Position};
use crate::specs::{GameSpecError, Validate};

use super::{BoardSpec, MapSpec};

/// Player specs, determining a name, and important information to know their initial state
/// and transitions in the game.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlayerSpec {
    /// Player name, which doubles up as a unique identifier.
    pub name: String,

    /// A 2×2 rotation matrix representing this player's orientation on the board.
    /// All canonical move steps in piece specs are transformed by this matrix at
    /// blueprint-build time, so a single piece spec works correctly for every player.
    /// Required on a grid; a map has no orientation, so a player on a map has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<Direction>,

    /// Starting positions for all pieces for this player.
    pub starting_positions: Vec<PiecePositionSpec>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PiecePositionSpec {
    /// Piece name, which identifies a Piece, whose spec should be loaded.
    pub piece: String,

    /// Positions where the specified piece should be.
    pub positions: Vec<PositionSpec>,
}

/// A position as a spec writes it: a grid square's coordinates (`[4, 1]`), or the name of a
/// place or square (`"a"`, `"4,1"`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PositionSpec {
    Coordinates(Coordinates),
    Name(String),
}

impl PositionSpec {
    /// The name of the position.
    pub fn position(&self) -> Position {
        match self {
            PositionSpec::Coordinates(coordinates) => Position::from_coordinates(coordinates),
            PositionSpec::Name(name) => Position::from(name.as_str()),
        }
    }
}

#[cfg(test)]
impl PlayerSpec {
    /// Creates an empty player, with just a name. Used only for tests.
    pub fn from_name(name: &str) -> PlayerSpec {
        PlayerSpec {
            name: name.to_string(),
            direction: Some(vec![vec![1i16, 0i16], vec![0i16, 1i16]]), // identity — moves "up"
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
        match board.map.as_deref() {
            Some(map) => self.validate_on_map(piece_names, map),
            None => self.validate_on_grid(piece_names, board),
        }
    }
}

impl PlayerSpec {
    /// Refuses an unknown piece in the starting positions.
    fn validate_pieces(&self, piece_names: &HashSet<String>) -> Result<(), GameSpecError> {
        match self
            .starting_positions
            .iter()
            .find(|positions| !piece_names.contains(&positions.piece))
        {
            Some(positions) => Err(GameSpecError::UnknownPieceInStartingPosition(
                positions.piece.clone(),
            )),
            None => Ok(()),
        }
    }

    /// On a map: no direction, and every starting position the name of a place.
    fn validate_on_map(
        &self,
        piece_names: &HashSet<String>,
        map: &MapSpec,
    ) -> Result<(), GameSpecError> {
        if self.direction.is_some() {
            return Err(GameSpecError::NotOnAMap(format!(
                "player {} has a `direction`; a map has no orientation, so leave it out",
                self.name
            )));
        }
        self.validate_pieces(piece_names)?;
        let places: HashSet<&str> = map.places.iter().map(|p| p.name.as_str()).collect();
        for position in self.starting_positions.iter().flat_map(|s| &s.positions) {
            match position {
                PositionSpec::Coordinates(coordinates) => {
                    return Err(GameSpecError::NotOnAMap(format!(
                        "player {} starts a piece at the coordinates {coordinates:?}; a map's \
                         positions are place names",
                        self.name
                    )));
                }
                PositionSpec::Name(name) if !places.contains(name.as_str()) => {
                    return Err(GameSpecError::UnknownPosition(name.clone()));
                }
                PositionSpec::Name(_) => {}
            }
        }
        Ok(())
    }

    /// On a grid: an N×N direction, and every starting position an enabled square.
    fn validate_on_grid(
        &self,
        piece_names: &HashSet<String>,
        board: &BoardSpec,
    ) -> Result<(), GameSpecError> {
        // An absent direction is refused as the empty matrix.
        let no_direction = Direction::new();
        let direction = self.direction.as_ref().unwrap_or(&no_direction);

        // The direction matrix must be N×N for a board of N dimensions...
        let expected = board.dimensions.len();
        let found = if direction.len() != expected {
            Some(direction.len())
        } else {
            direction
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

        // ...with entries of -1, 0 or 1 (so turning a step can never overflow) and determinant
        // ±1 (rotation or rotation+reflection).
        let small_entries = direction
            .iter()
            .flatten()
            .all(|entry| (-1..=1).contains(entry));
        if !small_entries || !matches!(determinant(direction), Some(1) | Some(-1)) {
            return Err(GameSpecError::InvalidDirectionMatrix(direction.clone()));
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
                let position = match position {
                    PositionSpec::Coordinates(coordinates) => coordinates,
                    PositionSpec::Name(name) => {
                        if !is_enabled_square(name, board) {
                            return Err(GameSpecError::UnknownPosition(name.clone()));
                        }
                        continue;
                    }
                };

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

/// Whether `name` names a square of the grid that is not disabled.
fn is_enabled_square(name: &str, board: &BoardSpec) -> bool {
    let Some(coordinates) = Position::from(name).coordinates() else {
        return false;
    };
    coordinates.len() == board.dimensions.len()
        && coordinates
            .iter()
            .zip(&board.dimensions)
            .all(|(value, size)| value < size)
        && !board.disabled_positions.contains(&coordinates)
}
