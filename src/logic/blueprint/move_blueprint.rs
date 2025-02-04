use std::collections::HashMap;

use crate::logic::{Board, Piece};
use crate::shared::{into_position, ExtendedPosition, Position, Effect, BoardChange};
use crate::specs::MoveSpec;

// Basic states.
const EMPTY: &str = "EMPTY";
const ENEMY: &str = "ENEMY";
const ALLY: &str = "ALLY";

// Basic actions.
const MOVE: &str = "MOVE";

/// A `MoveBlueprint` is a factory for a single move. The move could be repeatable (i.e. Rooks),
/// but it's a single, discrete type of logic.
/// 
/// Actions are indexed by the state of the position.
#[derive(Clone, Debug)]
pub struct MoveBlueprint {
    pub id: u8,
    pub step: ExtendedPosition,
    pub actions: HashMap<String, String>,
    // pub repeat: ???
}

impl MoveBlueprint {
    pub fn from_spec(spec: MoveSpec) -> Self {
        let mut actions = HashMap::new();

        for action in spec.actions {
            actions.insert(action.state, action.action);
        }

        MoveBlueprint {
            id: spec.id,
            step: spec.step,
            actions
            // TODO: Parse the rest of the spec
        }
    }

    /// Calculates move based on a spec, and a board state.
    pub fn calculate_moves(&self, board: &Board, piece: &Piece, current_player: &String, source_position: &Position) -> Option<Vec<(Position, Effect)>> {
        // TODO: Consider move spec based on occupant.
        // TODO: Consider directional switches.
        // TODO: Consider repeating moves.
        // TODO: Consider special conditions.
        // TODO: Consider move dependencies.
        // TODO: Basically consider EVERYTHING!!
        
        let mut result_moves: Vec<(Position, Effect)> = Vec::new();
        
        // Component-wise addition of step.
        // TODO: Multiply step by player direction vector.
        let move_ext_pos: Vec<i16> = source_position.clone().iter().zip(self.step.iter()).map(|(&a, &b)| a as i16 + b).collect();

        // Check if new position is valid.
        if !board.is_position_valid(&move_ext_pos) {
            return None
        }

        // Get element at position
        let move_pos = into_position(&move_ext_pos);
        let position_occupant = board.position_occupant(&move_pos, current_player); 

        // Obtain state based on occupant.
        let state = if position_occupant.is_none() {
            // If there's no position occupant, it's an empty space.
            EMPTY
        } else if position_occupant.unwrap().player == *current_player {
            ALLY
        } else {
            // TODO: Custom states?
            ENEMY
        };

        // Grab action to execute.
        let action = self.actions.get(state);

        match action {
            Some(_) => {
                // Save move.
                // TODO: Create a structure to explain a move, based on the action, so that it can later be executed.
                // TODO: Do recursive moves as well.
                // TODO: Account for things other than "move".
                result_moves.push((move_pos.clone(), Effect {
                    action: MOVE.to_string(),
                    board_changes: vec![
                        BoardChange::clear(source_position),
                        BoardChange::set_piece(move_pos, piece.code.clone(), current_player.clone()),
                    ]
                }));
            },
            None => (),
        }

        Some(result_moves)
    }
}
