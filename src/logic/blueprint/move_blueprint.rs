use std::collections::{HashMap, HashSet};

use crate::logic::{Game, Piece};
use crate::shared::{
    apply_direction,
    into_position,
    Position,
    ExtendedPosition,
    Effect,
    EffectMetadata,
    BoardChange,
    EMPTY,
    NOT_EMPTY,
    ENEMY,
    ALLY,
    //
    FIRST_MOVE,
    DEPENDS_ON,
};
use crate::specs::{MoveSpec, PlayerSpec};

#[derive(Clone, Debug)]
pub struct Condition {
    pub code: String,
    pub move_id: u8,
}

#[derive(Clone, Debug)]
pub struct MoveRepeat {
    pub until: String,
    pub times: u8,
    pub loop_move: bool,
}

#[derive(Clone, Debug)]
pub struct Modifier {
    pub action: String,
    pub conditions: Vec<Condition>,

    // Options for the modifier. I.e. pieces to transform into.
    pub options: Vec<String>,
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

    // Conditions that must be met for the move to be valid.
    pub conditions: Vec<Condition>,

    // Modifiers for the move,containing the condition that must be met for
    // the modifier to be applied.
    pub modifiers: Vec<Modifier>,

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
        
        // Transform each piece's canonical step by the player's direction matrix so that
        // "forward" in the spec maps to the correct board direction for every player.
        for player_spec in players_spec {
            let player_step = apply_direction(&player_spec.direction, &spec.step);
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

        // Process conditions.
        let conditions = spec.conditions.iter()
            .map(|c| Condition {
                code: c.condition.clone(),
                move_id: c.move_id.unwrap_or(0u8),
            })
            .collect();

        let modifiers = spec.modifiers.iter()
            .map(|m| Modifier {
                action: m.action.clone(),
                conditions: m.conditions.iter()
                    .map(|c| Condition {
                        code: c.condition.clone(),
                        move_id: c.move_id.unwrap_or(0u8), // FIXME: This doesn't make sense, refactor.
                    })
                    .collect(),
                options: m.options.clone(),
            })
            .collect();

        // TODO: Process side effects.

        MoveBlueprint {
            id: spec.id,
            step,
            actions,
            modifiers,
            repeat_options: MoveRepeat {
                until,
                times,
                loop_move,
            },
            conditions
        }
    }

    /// Calculates move based on a spec, and a board state.
    /// We need to pass in a set of valid move ids to evaluate move dependencies.
    pub fn calculate_moves(&self, piece: &Piece, source_position: &Position, valid_move_ids: &HashSet<u8>, game: &Game) -> Option<Vec<(Position, Effect)>> {
        let mut iterations: u8 = 1;
        let mut current_source = source_position.clone();

        let mut all_moves: Vec<(Position, Effect)> = vec![];

        loop {
            let (moves, next_position) = self.calculate_single_move(piece, &current_source, valid_move_ids, game);

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
    pub fn calculate_single_move(&self, piece: &Piece, source_position: &Position, valid_move_ids: &HashSet<u8>, game: &Game) -> (Option<Vec<(Position, Effect)>>, Option<Position>) {
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
        
        let conditions_met = self.check_conditions(piece, source_position, valid_move_ids);
        if !conditions_met { return (None, None); }

        // Grab action to execute.
        let action = self.actions.get(state);

        match action {
            Some(action) => {
                // Save move.
                // TODO: Account for things other than "move".
                let mut piece = piece.clone();
                piece.total_moves += 1u16;

                // Check modifiers.
                let mut applied_modifier: Option<Modifier> = None;

                // Only one modifier is applied per move.
                // FIXME: This is an assumption that may change in the future.
                for modifier in &self.modifiers {
                    if modifier.conditions.iter().all(|c| game.check_position_condition(&target_position, &c.code)) {
                        applied_modifier = Some(modifier.clone());
                        break;
                    }
                }

                match applied_modifier {
                    Some(modifier) => {
                        // TODO: This depends on the action. For now only handling "transform".
                        result_moves.push((target_position.clone(), Effect {
                            action: modifier.action,
                            board_changes: vec![
                                BoardChange::clear(source_position),
                                BoardChange::set_piece(target_position.clone(), piece),
                            ],
                            metadata: Some(EffectMetadata::Options(modifier.options)),
                        }));
                    },
                    None => {
                        result_moves.push((target_position.clone(), Effect {
                            action: action.to_string(),
                            board_changes: vec![
                                BoardChange::clear(source_position),
                                BoardChange::set_piece(target_position.clone(), piece),
                            ],
                            metadata: None,
                        }));
                    }
                }

                
            },
            None => (),
        }

        (Some(result_moves), Some(target_position))
    }

    // ---------------------------------------------------------------------
    // Utility functions
    // ---------------------------------------------------------------------

    // Check if all conditions for a move are met.
    pub fn check_conditions(&self, piece: &Piece, _source_position: &Position, valid_move_ids: &HashSet<u8>) -> bool {
        for condition in &self.conditions {
            if condition.code == FIRST_MOVE {
                if piece.total_moves > 0u16 {
                    return false;
                }
            } else if condition.code == DEPENDS_ON {
                if !valid_move_ids.contains(&condition.move_id) {
                    return false;
                }
            }

            // TODO: Implement custom condition checking?
        }
        
        true
    }
}
