use crate::logic::{Board, Piece};
use crate::shared::{into_position, ExtendedPosition, Position};
use crate::specs::MoveSpec;

/// A `MoveBlueprint` is a factory for a single move. The move could be repeatable (i.e. Rooks),
/// but it's a single, discrete type of logic.
#[derive(Debug)]
pub struct MoveBlueprint {
    pub id: u8,
    pub step: ExtendedPosition
    // pub repeat: ???
}

impl MoveBlueprint {
    pub fn from_spec(spec: MoveSpec) -> Self {
        MoveBlueprint {
            id: spec.id,
            step: spec.step
            // TODO: Parse the rest of the spec
        }
    }

    /// Calculates move based on a spec, and a board state.
    pub fn calculate_moves(&self, board: &Board, piece: &Piece, position: &Position) -> Option<Vec<Position>> {
        let mut viable_moves: Vec<Position> = Vec::new();
        
        // Component-wise addition of step
        let move_ext_pos: Vec<i16> = position.iter().zip(self.step.iter()).map(|(&a, &b)| a as i16 + b).collect();

        // Check if new position is valid.
        if !board.is_position_valid(&move_ext_pos) {
            return None
        }

        // TODO: Get element at position
        let move_pos = into_position(&move_ext_pos);
        let piece_at_position = board.piece_at_position(&move_pos);

        match piece_at_position {
            Some(_) => {
                // TODO: Enemy / ally logic
                ()
            }
            None => viable_moves.push(move_pos)
        }


        // FIXME: Placeholder solution
        Some(viable_moves)
    }
}
