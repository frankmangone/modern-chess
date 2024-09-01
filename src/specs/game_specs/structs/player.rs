use serde::{Deserialize, Serialize};

use super::position::Position;

/// Player specs, determining a name, and important information to know their initial state
/// and transitions in the game.
#[derive(Debug, Deserialize, Serialize)]
pub struct PlayerSpec {
    /// Player name, which doubles up as a unique identifier.
    pub name: String,

    /// Direction, which just tells us which is the "positive" direction for this player,
    /// for each direction axis. Possible values for each index are 1 and -1.
    pub direction: Vec<i8>,

    /// Starting positions for all pieces for this player.
    pub starting_positions: Vec<PiecePositionSpec>,
}

#[derive(Debug, Deserialize, Serialize)]
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
            direction: vec![],
            starting_positions: vec![]
        }
    }
}
