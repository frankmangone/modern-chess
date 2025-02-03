use crate::logic::{Board, Piece};
use crate::shared::{Position, Move};
use crate::specs::PieceSpec;

use super::move_blueprint::MoveBlueprint;

/// A `PieceBlueprint` is essentially a factory for piece movement calculation.
/// It stores the set of rules used to calculate the available moves, but without knowledge
/// of the actual piece position.
/// 
/// A piece blueprint is associated with a piece code, and contains a list of move blueprints.
/// Each move blueprint is a factory for a single move.
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

    /// Calculates the moves associated with each move blueprint.
    pub fn calculate_moves(&self, board: &Board, piece: &Piece, current_player: &String, position: &Position) -> Option<Vec<Move>> {
        let mut moves: Vec<Move> = Vec::new();
        
        for move_blueprint in &self.move_blueprints {
            match move_blueprint.calculate_moves(board, piece, current_player, position) {
                Some(value) => {
                    let mut value = value.clone();
                    moves.append(&mut value)
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
