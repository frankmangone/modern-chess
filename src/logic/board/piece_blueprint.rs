use crate::logic::Board;
use crate::shared::Position;
use crate::specs::PieceSpec;

use super::move_blueprint::MoveBlueprint;

/// A `PieceBlueprint` is essentially a factory for piece movement calculation.
/// It stores the set of rules used to calculate the available moves, but without knowledge
/// of the actual piece position.
#[derive(Debug)]
pub struct PieceBlueprint {
    pub move_blueprints: Vec<MoveBlueprint>
}

impl PieceBlueprint {
    pub fn from_spec(spec: PieceSpec) -> Self {
        PieceBlueprint {
            move_blueprints: spec.moves.into_iter().map(|x| MoveBlueprint::from_spec(x)).collect()
        }
    }

    pub fn calculate_moves(&self, board: &Board) -> Option<Vec<Position>> {
        let mut moves: Vec<Position> = Vec::new();
        
        for blueprint in &self.move_blueprints {
            match blueprint.calculate_moves(board) {
                Some(value) => {
                    let mut value = value.clone();
                    moves.append(&mut value)
                },
                None => (),
            };
        }

        None
    }
}
