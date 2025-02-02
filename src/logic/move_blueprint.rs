use crate::logic::{Board, Piece};
use crate::shared::{into_position, ExtendedPosition, Position, PositionOccupant};
use crate::specs::{MoveSpec, ActionSpec};

/// A `MoveBlueprint` is a factory for a single move. The move could be repeatable (i.e. Rooks),
/// but it's a single, discrete type of logic.
#[derive(Debug)]
pub struct MoveBlueprint {
    pub id: u8,
    pub step: ExtendedPosition,
    // pub actions: ActionSpec,
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
    pub fn calculate_moves(&self, board: &Board, piece: &Piece, current_player: &String, position: &Position) -> Option<Vec<Position>> {
        // TODO: Consider move spec based on occupant.
        // TODO: Consider directional switches.
        // TODO: Consider repeating moves.
        // TODO: Consider special conditions.
        // TODO: Consider move dependencies.
        
        let mut viable_moves: Vec<Position> = Vec::new();
        
        // Component-wise addition of step.
        let move_ext_pos: Vec<i16> = position.iter().zip(self.step.iter()).map(|(&a, &b)| a as i16 + b).collect();

        // Check if new position is valid.
        if !board.is_position_valid(&move_ext_pos) {
            return None
        }

        // Get element at position
        let move_pos = into_position(&move_ext_pos);
        let position_occupant = board.position_occupant(&move_pos, current_player); 

        match position_occupant {
            // TODO: Execute logic based on specs
            PositionOccupant::Empty => {
                // FIXME: MOVE
                viable_moves.push(move_pos)
            },
            PositionOccupant::Ally(_piece) => {
                // FIXME: DO NOTHING
                ()
            }
            PositionOccupant::Enemy(_piece, _player) => {
                // FIXME: TAKE
                viable_moves.push(move_pos)
            }
        }

        Some(viable_moves)
    }
}
