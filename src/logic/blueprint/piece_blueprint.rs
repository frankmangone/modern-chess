use std::collections::HashMap;

use crate::logic::{Board, Piece};
use crate::shared::{Position, Effect};
use crate::specs::PieceSpec;

use super::move_blueprint::MoveBlueprint;

/// A `PieceBlueprint` is essentially a factory for piece movement calculation.
/// It stores the set of rules used to calculate the available moves, but without knowledge
/// of the actual piece position.
/// 
/// A piece blueprint is associated with a piece code, and contains a list of move blueprints.
/// Each move blueprint is a factory for a single move.
#[derive(Clone, Debug)]
pub struct PieceBlueprint {
    pub move_blueprints: Vec<MoveBlueprint>
}

impl PieceBlueprint {
    pub fn from_spec(spec: PieceSpec) -> Self {
        PieceBlueprint {
            move_blueprints: spec.moves.into_iter().map(|x| MoveBlueprint::from_spec(x)).collect()
        }
    }

    /// Calculates the moves associated with each move blueprint.
    pub fn calculate_moves(&self, board: &Board, piece: &Piece, current_player: &String, position: &Position) -> Option<HashMap<Position, Effect>> {
        let mut moves: HashMap<Position, Effect> = HashMap::new();
        
        for move_blueprint in &self.move_blueprints {
            match move_blueprint.calculate_moves(board, piece, current_player, position) {
                Some(value) => {
                    // `value` is a vector of (Position, Vec<Effect>), where the position is the "target" position
                    // and the vector is the list of effects to be executed.
                    value.iter().for_each(|(pos, effects)| {
                        moves.insert(pos.clone(), effects.clone());
                    });
                },
                None => (),
            };
        }

        if moves.len() > 0 {
            Some(moves)
        } else {
            None
        }
    }
}
