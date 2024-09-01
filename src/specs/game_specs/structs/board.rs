use serde::{Deserialize, Serialize};
use super::position::Position;

/// Board spec, mostly consisting of layout specifications.
#[derive(Debug, Deserialize, Serialize)]
pub struct BoardSpec {
    /// The base dimensions of the board. For instance, chess should have `vec![8u8, 8u8]`.
    dimensions: Vec<u8>,

    /// A set of positions that are disabled in the domain specified by the dimensions.
    #[serde(default)]
    disabled_positions: Option<Vec<Position>>
}

#[cfg(test)]
impl BoardSpec {
    /// Default board is just a chess board. Used for tests.
    pub fn default() -> Self {
        BoardSpec {
            dimensions: vec![8u8, 8u8], 
            disabled_positions: None
        }
    }
}