pub mod player;
pub mod turns;
pub mod position;

pub use position::Position;
pub use player::PlayerSpec;
pub use turns::TurnSpec;

//

use serde::{Deserialize, Serialize, Deserializer};
use std::collections::HashSet;

/// Board spec, mostly consisting of layout specifications.
#[derive(Debug, Deserialize, Serialize)]
pub struct BoardSpec {
    /// The base dimensions of the board. For instance, chess should have `vec![8u8, 8u8]`.
    pub dimensions: Vec<u8>,

    /// A set of positions that are disabled in the domain specified by the dimensions.
    #[serde(default, deserialize_with = "deserialize_disabled_positions")]
    pub disabled_positions: HashSet<Position>
}

/// Custom deserialization function for `disabled_positions`.
fn deserialize_disabled_positions<'de, D>(
    deserializer: D,
) -> Result<HashSet<Position>, D::Error>
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
    /// Default board is just a chess board. Used for tests.
    pub fn default() -> Self {
        BoardSpec {
            dimensions: vec![8u8, 8u8], 
            disabled_positions: HashSet::new()
        }
    }
}
