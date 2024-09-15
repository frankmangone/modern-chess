use crate::logic::Board;
use crate::shared::Position;
use crate::specs::MoveSpec;

/// A `MoveBlueprint` is a factory for a single move. The move could be repeatable (i.e. Rooks),
/// but it's a single, discrete type of logic.
#[derive(Debug)]
pub struct MoveBlueprint {
    pub step: Position
    // pub repeat: ???
}

impl MoveBlueprint {
    pub fn from_spec(spec: MoveSpec) -> Self {
        // TODO: Parse spec
        MoveBlueprint {
            step: vec![]
        }
    }

    pub fn calculate_moves(&self, board: &Board) -> Option<Vec<Position>> {
        // TODO: Calculate moves
        None
    }
}
