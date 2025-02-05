use std::collections::HashMap;

use crate::logic::{Game, Piece};
use crate::shared::{
    into_position,
    Position,
    ExtendedPosition,
    Effect,
    BoardChange,
    EMPTY,
    NOT_EMPTY,
    ENEMY,
    ALLY,
    MOVE
};
use crate::specs::{MoveSpec, PlayerSpec};

#[derive(Clone, Debug)]
pub struct MoveRepeat {
    pub until: String,
    pub times: u8,
    pub loop_move: bool,
}
/// A `MoveBlueprint` is a factory for a single move. The move could be repeatable (i.e. Rooks),
/// but it's a single, discrete type of logic.
/// 
/// Actions are indexed by the state of the position.
#[derive(Clone, Debug)]
pub struct MoveBlueprint {
    pub id: u8,
    pub step: HashMap<String, ExtendedPosition>, // player -> step
    pub actions: HashMap<String, String>,

    // Number of times to repeat the move. `0u8` means repeat indefinitely.
    // Defaults to `1u8`.
    pub repeat_options: MoveRepeat
}

impl MoveBlueprint {
    pub fn from_spec(spec: MoveSpec, players_spec: Vec<PlayerSpec>) -> Self {
        let mut actions = HashMap::new();

        for action in spec.actions {
            actions.insert(action.state, action.action);
        }

        let mut step: HashMap<String, ExtendedPosition> = HashMap::new();
        
        // Some pieces - like Pawns - have a step that is different for each player.
        // This is handled by having a `step` that is a HashMap from player name to step.
        for player_spec in players_spec {
            let player_step: Vec<i16> = spec.step.iter()
                .zip(player_spec.direction.iter())
                .map(|(&s, &d)| s * d)
                .collect();
            step.insert(player_spec.name, player_step);
        }

        // Process repeat information. 
        // `until` defaults to `NOT_EMPTY`.
        // `times` defaults to `1u8`.
        // `loop_move` defaults to `false`.
        let (until, times, loop_move) = match spec.repeat.clone() {
            Some(repeat) => {
                (
                    repeat.until.clone().unwrap_or(NOT_EMPTY.to_string()),
                    repeat.times.unwrap_or(1u8),
                    repeat.loop_move,
                )
            },
            None => (NOT_EMPTY.to_string(), 1u8, false)
        };

        MoveBlueprint {
            id: spec.id,
            step,
            actions,
            repeat_options: MoveRepeat {
                until,
                times,
                loop_move,
            }
            // TODO: Parse the rest of the spec
        }
    }

    /// Calculates move based on a spec, and a board state.
    pub fn calculate_moves(&self, piece: &Piece, source_position: &Position, game: &Game) -> Option<Vec<(Position, Effect)>> {
        let mut iterations: u8 = 1;
        let mut current_source = source_position.clone();

        let mut all_moves: Vec<(Position, Effect)> = vec![];

        loop {
            let (moves, next_position) = self.calculate_single_move(piece, &current_source, game);

            if let Some(moves) = moves {
                all_moves.extend(moves);
            }

            let invalid_next_position = next_position.is_none();
            let max_iterations_reached = !self.repeat_options.loop_move && iterations >= self.repeat_options.times;
            let until_condition_met = match &next_position {
                Some(pos) => game.check_position_condition(pos, &self.repeat_options.until),
                None => false
            };

            if invalid_next_position || max_iterations_reached || until_condition_met {
                break;
            }


            current_source = next_position.unwrap();

            iterations += 1u8;
        }

        if all_moves.len() != 0 {
            Some(all_moves)
        } else {
            None
        }
    }

    /// Calculates a single move based on a spec, and a board state. Used for recursive moves.
    /// First return value are the moves for this evaluation, second is the position to recurse to.
    /// If the latter is `None`, then the move is not repeatable.
    pub fn calculate_single_move(&self, piece: &Piece, source_position: &Position, game: &Game) -> (Option<Vec<(Position, Effect)>>, Option<Position>) {
        // TODO: Consider special conditions.
        // TODO: Consider move dependencies.
        // TODO: Basically consider EVERYTHING!!
        
        let current_player = &game.current_player();

        let mut result_moves: Vec<(Position, Effect)> = Vec::new();
        
        // Component-wise addition of step. The step is already multiplied by the player direction vector.
        let target_position: Vec<i16> = source_position.clone().iter()
            .zip(self.step.get(current_player).unwrap().iter()).map(|(&a, &b)| a as i16 + b)
            .collect();

        // Check if target position is valid.
        if !game.board.is_position_valid(&target_position) {
            return (None, None);
        }

        // Get element at position
        let target_position = into_position(&target_position);
        let target_position_piece = game.piece_at_position(&target_position);

        // Obtain state based on occupant.
        let state = if target_position_piece.is_none() {
            // If there's no piece at target position, it's an empty space.
            EMPTY
        } else if target_position_piece.unwrap().player == *current_player {
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
                // TODO: Do recursive moves as well.
                // TODO: Account for things other than "move".
                result_moves.push((target_position.clone(), Effect {
                    action: MOVE.to_string(),
                    board_changes: vec![
                        BoardChange::clear(source_position),
                        BoardChange::set_piece(target_position.clone(), piece.code.clone(), current_player.clone()),
                    ]
                }));
            },
            None => (),
        }

        (Some(result_moves), Some(target_position))
    }
}
