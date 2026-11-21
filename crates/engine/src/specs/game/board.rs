pub use super::PlayerSpec;
pub use super::TurnSpec;

//

use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashSet;

use crate::shared::Position;
use crate::specs::GameSpecError;

/// The most dimensions a board may have.
pub const MAX_DIMENSIONS: usize = 8;

/// The most positions a board may have, counting disabled ones. Every position is listed when
/// moves and scores are worked out, so a board must stay small enough to list.
pub const MAX_POSITIONS: u64 = 10_000;

/// Board spec, mostly consisting of layout specifications.
#[derive(Debug, Deserialize, Serialize)]
pub struct BoardSpec {
    /// The base dimensions of the board. For instance, chess should have `vec![8u8, 8u8]`.
    pub dimensions: Position,

    /// A set of positions that are disabled in the domain specified by the dimensions.
    #[serde(default, deserialize_with = "deserialize_disabled_positions")]
    pub disabled_positions: HashSet<Position>,
}

impl BoardSpec {
    /// Refuses a board with no dimensions, a dimension of size 0, more than `MAX_DIMENSIONS`
    /// dimensions or more than `MAX_POSITIONS` positions.
    pub fn validate_size(&self) -> Result<(), GameSpecError> {
        let refused = || Err(GameSpecError::InvalidBoardSize(self.dimensions.clone()));
        if self.dimensions.is_empty() || self.dimensions.len() > MAX_DIMENSIONS {
            return refused();
        }
        let mut positions: u64 = 1;
        for &size in &self.dimensions {
            if size == 0 {
                return refused();
            }
            positions = positions.saturating_mul(u64::from(size));
        }
        if positions > MAX_POSITIONS {
            return refused();
        }
        Ok(())
    }
}

/// Custom deserialization function for `disabled_positions`.
fn deserialize_disabled_positions<'de, D>(deserializer: D) -> Result<HashSet<Position>, D::Error>
where
    D: Deserializer<'de>,
{
    let vec = Option::<Vec<Position>>::deserialize(deserializer)?;
    let mut map = HashSet::new();

    if let Some(positions) = vec {
        for position in positions {
            map.insert(position);
        }
    }

    Ok(map)
}

#[cfg(test)]
impl BoardSpec {
    /// Default board is just a chess board. Used for tests only.
    pub fn default() -> Self {
        BoardSpec {
            dimensions: vec![8u8, 8u8],
            disabled_positions: HashSet::new(),
        }
    }
}
