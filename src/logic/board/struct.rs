use std::collections::HashSet;

use crate::specs::BoardSpec;
use crate::shared::Position;

#[derive(Debug)]
pub struct Board {
    pub dimensions: Vec<u8>,
    pub disabled_positions: HashSet<Position>
}

impl Board {
    pub fn from_spec(spec: BoardSpec) -> Board {
        Board {
            dimensions: spec.dimensions,
            disabled_positions: spec.disabled_positions
        }
    }
}