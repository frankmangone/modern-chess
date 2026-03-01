use std::collections::{HashMap, HashSet};

use crate::logic::{Game, Piece, PieceState};
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
    CHECK_STATE,
    PIECE_FIRST_MOVE,
    ROOK_FIRST_MOVE,
    PATH_EMPTY,
    NOT_ATTACKED,
    PATH_NOT_ATTACKED,
    SET_STATE,
    MOVE,
    CAPTURE,
};
use crate::specs::{MoveSpec, PlayerSpec};

#[derive(Clone, Debug)]
pub struct Condition {
    pub code: String,

    // For DEPENDS_ON: the id of the move blueprint that must have produced a valid move.
    // None for all other condition types where move_id has no meaning.
    pub move_id: Option<u8>,

    // For CHECK_STATE: the name of the state flag to look for.
    pub state: Option<String>,

    // For CHECK_STATE and PIECE_FIRST_MOVE: relative offset from the source position,
    // pre-transformed per player at blueprint-build time (same convention as `step`).
    pub position: Option<HashMap<String, ExtendedPosition>>,
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

/// Runtime representation of a side effect, with relative positions pre-transformed
/// per player at blueprint-build time.
#[derive(Clone, Debug)]
pub struct SideEffectBlueprint {
    /// "SET_STATE", "CAPTURE", "MOVE" (move-piece), etc.
    pub action: String,
    /// For SET_STATE: the state flag name to set on the moved piece.
    pub state: Option<String>,
    /// For SET_STATE: countdown duration. None = permanent (Blank flag).
    pub duration: Option<u8>,
    /// For MOVE side effect: source position (relative to the moving piece's source), per player.
    pub from: Option<HashMap<String, ExtendedPosition>>,
    /// For MOVE side effect: destination position, per player.
    pub to: Option<HashMap<String, ExtendedPosition>>,
    /// For CAPTURE side effect: position to clear (relative to the moving piece's source), per player.
    pub target: Option<HashMap<String, ExtendedPosition>>,
}

/// Runtime representation of a move action, bundling the action string with
/// optional action-level conditions (gates whether this action fires) and
/// action-level side effects (only applied when this action fires).
#[derive(Clone, Debug)]
pub struct ActionBlueprint {
    pub action: String,
    pub conditions: Vec<Condition>,
    pub side_effects: Vec<SideEffectBlueprint>,
}

/// A `MoveBlueprint` is a factory for a single move. The move could be repeatable (i.e. Rooks),
/// but it's a single, discrete type of logic.
///
/// Actions are indexed by the state of the position.
#[derive(Clone, Debug)]
pub struct MoveBlueprint {
    pub id: u8,
    pub step: HashMap<String, ExtendedPosition>, // player -> step
    pub actions: HashMap<String, ActionBlueprint>,

    // Conditions that must be met for the move to be valid.
    pub conditions: Vec<Condition>,

    // Modifiers for the move, containing the condition that must be met for
    // the modifier to be applied.
    pub modifiers: Vec<Modifier>,

    // Move-level side effects: applied whenever this move executes, regardless of which action fires.
    pub side_effects: Vec<SideEffectBlueprint>,

    // Number of times to repeat the move. `0u8` means repeat indefinitely.
    // Defaults to `1u8`.
    pub repeat_options: MoveRepeat,
}

impl MoveBlueprint {
    pub fn from_spec(spec: MoveSpec, players_spec: Vec<PlayerSpec>) -> Self {
        // Helper: transform a raw [i8; 2] relative position for all players.
        fn transform_pos(
            raw: Option<[i8; 2]>,
            players: &[PlayerSpec],
        ) -> Option<HashMap<String, ExtendedPosition>> {
            raw.map(|r| {
                players.iter().map(|p| {
                    let vec: ExtendedPosition = vec![r[0] as i16, r[1] as i16];
                    (p.name.clone(), apply_direction(&p.direction, &vec))
                }).collect()
            })
        }

        // Helper: convert a ConditionSpec slice into Condition structs, transforming
        // any relative positions per player.
        fn build_conditions(
            specs: &[crate::specs::game::piece::condition::ConditionSpec],
            players: &[PlayerSpec],
        ) -> Vec<Condition> {
            specs.iter().map(|c| {
                let position = c.position.map(|rel| {
                    players.iter().map(|p| {
                        let canon = vec![rel[0] as i16, rel[1] as i16];
                        (p.name.clone(), apply_direction(&p.direction, &canon))
                    }).collect()
                });
                Condition {
                    code: c.condition.clone(),
                    move_id: c.move_id,
                    state: c.state.clone(),
                    position,
                }
            }).collect()
        }

        // Build actions map: state -> ActionBlueprint.
        let mut actions = HashMap::new();
        for action_spec in spec.actions {
            let action_conditions = build_conditions(&action_spec.conditions, &players_spec);
            let action_side_effects: Vec<SideEffectBlueprint> = action_spec.side_effects.iter()
                .map(|se| SideEffectBlueprint {
                    action: se.action.clone(),
                    state: se.state.clone(),
                    duration: se.duration,
                    from: transform_pos(se.from, &players_spec),
                    to: transform_pos(se.to, &players_spec),
                    target: transform_pos(se.target, &players_spec),
                })
                .collect();
            actions.insert(action_spec.state, ActionBlueprint {
                action: action_spec.action,
                conditions: action_conditions,
                side_effects: action_side_effects,
            });
        }

        // Transform each piece's canonical step by the player's direction matrix so that
        // "forward" in the spec maps to the correct board direction for every player.
        let mut step: HashMap<String, ExtendedPosition> = HashMap::new();
        for player_spec in &players_spec {
            let player_step = apply_direction(&player_spec.direction, &spec.step);
            step.insert(player_spec.name.clone(), player_step);
        }

        // Process repeat information.
        let (until, times, loop_move) = match spec.repeat.clone() {
            Some(repeat) => (
                repeat.until.clone().unwrap_or(NOT_EMPTY.to_string()),
                repeat.times.unwrap_or(1u8),
                repeat.loop_move,
            ),
            None => (NOT_EMPTY.to_string(), 1u8, false),
        };

        // Process move-level conditions.
        let conditions = build_conditions(&spec.conditions, &players_spec);

        // Process modifiers.
        let modifiers = spec.modifiers.iter()
            .map(|m| Modifier {
                action: m.action.clone(),
                conditions: m.conditions.iter()
                    .map(|c| Condition {
                        code: c.condition.clone(),
                        move_id: None, // modifier conditions never use DEPENDS_ON
                        state: c.state.clone(),
                        position: None,
                    })
                    .collect(),
                options: m.options.clone(),
            })
            .collect();

        // Process move-level side effects.
        let side_effects: Vec<SideEffectBlueprint> = spec.side_effects.iter()
            .map(|se| SideEffectBlueprint {
                action: se.action.clone(),
                state: se.state.clone(),
                duration: se.duration,
                from: transform_pos(se.from, &players_spec),
                to: transform_pos(se.to, &players_spec),
                target: transform_pos(se.target, &players_spec),
            })
            .collect();

        MoveBlueprint {
            id: spec.id,
            step,
            actions,
            conditions,
            modifiers,
            side_effects,
            repeat_options: MoveRepeat { until, times, loop_move },
        }
    }

    /// Calculates move based on a spec, and a board state.
    /// We need to pass in a set of valid move ids to evaluate move dependencies.
    pub fn calculate_moves(
        &self,
        piece: &Piece,
        source_position: &Position,
        valid_move_ids: &HashSet<u8>,
        game: &Game,
    ) -> Option<Vec<(Position, Effect)>> {
        let mut iterations: u8 = 1;
        let mut current_source = source_position.clone();
        let mut all_moves: Vec<(Position, Effect)> = vec![];

        loop {
            let (moves, next_position) =
                self.calculate_single_move(piece, &current_source, valid_move_ids, game);

            if let Some(moves) = moves {
                all_moves.extend(moves);
            }

            let invalid_next_position = next_position.is_none();
            let max_iterations_reached =
                !self.repeat_options.loop_move && iterations >= self.repeat_options.times;
            let until_condition_met = match &next_position {
                Some(pos) => game.check_position_condition(pos, &self.repeat_options.until),
                None => false,
            };

            if invalid_next_position || max_iterations_reached || until_condition_met {
                break;
            }

            current_source = next_position.unwrap();
            iterations += 1u8;
        }

        if all_moves.len() != 0 { Some(all_moves) } else { None }
    }

    /// Calculates a single move based on a spec, and a board state. Used for recursive moves.
    /// First return value are the moves for this evaluation, second is the position to recurse to.
    /// If the latter is `None`, then the move is not repeatable.
    pub fn calculate_single_move(
        &self,
        piece: &Piece,
        source_position: &Position,
        valid_move_ids: &HashSet<u8>,
        game: &Game,
    ) -> (Option<Vec<(Position, Effect)>>, Option<Position>) {
        let current_player = &game.current_player();

        let mut result_moves: Vec<(Position, Effect)> = Vec::new();

        // Component-wise addition of step (already transformed for this player).
        let target_position: Vec<i16> = source_position.iter()
            .zip(self.step.get(current_player).unwrap().iter())
            .map(|(&a, &b)| a as i16 + b)
            .collect();

        if !game.board.is_position_valid(&target_position) {
            return (None, None);
        }

        let target_position = into_position(&target_position);
        let target_position_piece = game.piece_at_position(&target_position);

        let state = if target_position_piece.is_none() {
            EMPTY
        } else if target_position_piece.unwrap().player == *current_player {
            ALLY
        } else {
            ENEMY
        };

        // Check move-level conditions.
        let conditions_met = self.check_conditions(piece, source_position, valid_move_ids, game);
        if !conditions_met { return (None, None); }

        // Look up the action blueprint for the current board state.
        if let Some(action_bp) = self.actions.get(state) {
            // Check action-level conditions (gates whether this specific action fires).
            let action_conds_met =
                self.evaluate_conditions(&action_bp.conditions, piece, source_position, valid_move_ids, game);

            if action_conds_met {
                let mut moved_piece = piece.clone();
                moved_piece.total_moves += 1u16;

                // Collect extra board changes from side effects.
                // Move-level side effects always fire; action-level side effects only when
                // this action fires (i.e., here, inside action_conds_met == true).
                let mut extra_changes: Vec<BoardChange> = Vec::new();

                for se in self.side_effects.iter().chain(action_bp.side_effects.iter()) {
                    match se.action.as_str() {
                        SET_STATE => {
                            // Attach a state flag to the moved piece itself.
                            if let Some(flag) = &se.state {
                                let value = match se.duration {
                                    Some(d) => PieceState::Uint(d as u16),
                                    None => PieceState::Blank,
                                };
                                moved_piece.state.insert(flag.clone(), value);
                            }
                        },
                        CAPTURE => {
                            // Remove a piece at a relative position (e.g. en passant).
                            if let Some(tgt_map) = &se.target {
                                if let Some(rel) = tgt_map.get(current_player) {
                                    let abs: Vec<i16> = source_position.iter()
                                        .zip(rel.iter())
                                        .map(|(&s, &r)| s as i16 + r)
                                        .collect();
                                    if game.board.is_position_valid(&abs) {
                                        let capture_pos = into_position(&abs);
                                        extra_changes.push(BoardChange::clear(&capture_pos));
                                    }
                                }
                            }
                        },
                        MOVE => {
                            // Move another piece from a relative `from` to a relative `to`
                            // (e.g. castling rook). If no piece is found at `from`, skip silently.
                            if let (Some(from_map), Some(to_map)) = (&se.from, &se.to) {
                                if let (Some(from_rel), Some(to_rel)) =
                                    (from_map.get(current_player), to_map.get(current_player))
                                {
                                    let from_abs: Vec<i16> = source_position.iter()
                                        .zip(from_rel.iter())
                                        .map(|(&s, &r)| s as i16 + r)
                                        .collect();
                                    let to_abs: Vec<i16> = source_position.iter()
                                        .zip(to_rel.iter())
                                        .map(|(&s, &r)| s as i16 + r)
                                        .collect();
                                    if game.board.is_position_valid(&from_abs)
                                        && game.board.is_position_valid(&to_abs)
                                    {
                                        let from_pos = into_position(&from_abs);
                                        let to_pos = into_position(&to_abs);
                                        if let Some(other) = game.piece_at_position(&from_pos) {
                                            extra_changes.push(BoardChange::clear(&from_pos));
                                            extra_changes.push(BoardChange::set_piece(to_pos, other));
                                        }
                                    }
                                }
                            }
                        },
                        _ => {},
                    }
                }

                // Check for a modifier (e.g. pawn promotion).
                let mut applied_modifier: Option<Modifier> = None;
                for modifier in &self.modifiers {
                    if modifier.conditions.iter()
                        .all(|c| game.check_position_condition(&target_position, &c.code))
                    {
                        applied_modifier = Some(modifier.clone());
                        break;
                    }
                }

                let mut board_changes = vec![
                    BoardChange::clear(source_position),
                    BoardChange::set_piece(target_position.clone(), moved_piece),
                ];
                board_changes.extend(extra_changes);

                match applied_modifier {
                    Some(modifier) => {
                        result_moves.push((target_position.clone(), Effect {
                            action: modifier.action,
                            board_changes,
                            metadata: Some(EffectMetadata::Options(modifier.options)),
                        }));
                    },
                    None => {
                        result_moves.push((target_position.clone(), Effect {
                            action: action_bp.action.clone(),
                            board_changes,
                            metadata: None,
                        }));
                    },
                }
            }
            // If action conditions fail: no move added, but position is still valid for looping.
        }

        (Some(result_moves), Some(target_position))
    }

    /// Returns the set of squares threatened by this move blueprint from `source_position`,
    /// ignoring move-level conditions (threat is about board geometry, not turn state).
    /// Only blueprints with an `ENEMY→CAPTURE` action contribute to the attack map.
    pub fn calculate_threats(
        &self,
        player: &str,
        source_position: &Position,
        game: &Game,
    ) -> HashSet<Position> {
        if !self.actions.contains_key(ENEMY) {
            return HashSet::new();
        }

        let mut threats = HashSet::new();
        let mut iterations: u8 = 1;
        let mut current_source = source_position.clone();

        loop {
            let Some(step) = self.step.get(player) else { break; };
            let target: Vec<i16> = current_source.iter()
                .zip(step.iter())
                .map(|(&s, &st)| s as i16 + st)
                .collect();

            if !game.board.is_position_valid(&target) { break; }
            let target_pos = into_position(&target);

            match game.piece_at_position(&target_pos) {
                Some(p) if p.player == player => break,           // ally blocks
                Some(_) => { threats.insert(target_pos); break; } // enemy: threatened, stop
                None => { threats.insert(target_pos.clone()); }   // empty: threatened, continue
            }

            let max_iterations_reached =
                !self.repeat_options.loop_move && iterations >= self.repeat_options.times;
            if max_iterations_reached { break; }

            current_source = target_pos;
            iterations += 1;
        }

        threats
    }

    // ---------------------------------------------------------------------
    // Utility functions
    // ---------------------------------------------------------------------

    /// Evaluates all move-level conditions. Returns `false` as soon as one fails.
    pub fn check_conditions(
        &self,
        piece: &Piece,
        source_position: &Position,
        valid_move_ids: &HashSet<u8>,
        game: &Game,
    ) -> bool {
        self.evaluate_conditions(&self.conditions, piece, source_position, valid_move_ids, game)
    }

    /// Core condition evaluator. Accepts any slice of conditions so it can be used for
    /// both move-level (`self.conditions`) and action-level (`action_bp.conditions`) checks.
    fn evaluate_conditions(
        &self,
        conditions: &[Condition],
        piece: &Piece,
        source_position: &Position,
        valid_move_ids: &HashSet<u8>,
        game: &Game,
    ) -> bool {
        let current_player = game.current_player();

        for condition in conditions {
            let passed = match condition.code.as_str() {

                FIRST_MOVE => piece.total_moves == 0,

                DEPENDS_ON => condition.move_id.map_or(false, |id| valid_move_ids.contains(&id)),

                CHECK_STATE => {
                    let (Some(pos_map), Some(state_name)) = (&condition.position, &condition.state) else {
                        return false;
                    };
                    let Some(offset) = pos_map.get(&current_player) else { return false; };
                    let abs: Vec<i16> = source_position.iter()
                        .zip(offset.iter())
                        .map(|(&s, &o)| s as i16 + o)
                        .collect();
                    if !game.board.is_position_valid(&abs) { return false; }
                    game.piece_at_position(&into_position(&abs))
                        .map_or(false, |p| p.state.contains_key(state_name.as_str()))
                },

                PIECE_FIRST_MOVE => {
                    let Some(pos_map) = &condition.position else { return false; };
                    let Some(offset) = pos_map.get(&current_player) else { return false; };
                    let abs: Vec<i16> = source_position.iter()
                        .zip(offset.iter())
                        .map(|(&s, &o)| s as i16 + o)
                        .collect();
                    if !game.board.is_position_valid(&abs) { return false; }
                    game.piece_at_position(&into_position(&abs))
                        .map_or(false, |p| p.total_moves == 0)
                },

                // ROOK_FIRST_MOVE: vacuously true when off-board or no piece found.
                // Used for castling where two conditions cover both rook positions
                // (one will be off-board / empty for each player).
                ROOK_FIRST_MOVE => {
                    let Some(pos_map) = &condition.position else { return false; };
                    let Some(offset) = pos_map.get(&current_player) else { return false; };
                    let abs: Vec<i16> = source_position.iter()
                        .zip(offset.iter())
                        .map(|(&s, &o)| s as i16 + o)
                        .collect();
                    if !game.board.is_position_valid(&abs) { true } // off-board → pass
                    else {
                        game.piece_at_position(&into_position(&abs))
                            .map_or(true, |p| p.total_moves == 0) // no piece → pass
                    }
                },

                PATH_EMPTY => {
                    let Some(step) = self.step.get(&current_player) else { return false; };
                    let max_steps = step.iter().map(|&s| s.abs()).max().unwrap_or(0);
                    if max_steps <= 1 { true }
                    else {
                        let unit: Vec<i16> = step.iter().map(|&s| s.signum()).collect();
                        (1..max_steps).all(|i| {
                            let pos: Vec<i16> = source_position.iter()
                                .zip(unit.iter())
                                .map(|(&s, &u)| s as i16 + u * i)
                                .collect();
                            game.board.is_position_valid(&pos)
                                && game.piece_at_position(&into_position(&pos)).is_none()
                        })
                    }
                },

                NOT_ATTACKED => {
                    let Some(step) = self.step.get(&current_player) else { return false; };
                    let raw: Vec<i16> = source_position.iter()
                        .zip(step.iter())
                        .map(|(&s, &st)| s as i16 + st)
                        .collect();
                    if !game.board.is_position_valid(&raw) { return false; }
                    let target = into_position(&raw);
                    !game.players.iter()
                        .filter(|p| **p != current_player)
                        .any(|opp| game.attacked_by(opp).contains(&target))
                },

                PATH_NOT_ATTACKED => {
                    let Some(step) = self.step.get(&current_player) else { return false; };
                    let max_steps = step.iter().map(|&s| s.abs()).max().unwrap_or(0);
                    if max_steps <= 0 { true }
                    else {
                        let unit: Vec<i16> = step.iter().map(|&s| s.signum()).collect();
                        let opponent_attacks: HashSet<Position> = game.players.iter()
                            .filter(|p| **p != current_player)
                            .flat_map(|opp| game.attacked_by(opp))
                            .collect();
                        (1..=max_steps).all(|i| {
                            let pos: Vec<i16> = source_position.iter()
                                .zip(unit.iter())
                                .map(|(&s, &u)| s as i16 + u * i)
                                .collect();
                            game.board.is_position_valid(&pos)
                                && !opponent_attacks.contains(&into_position(&pos))
                        })
                    }
                },

                _ => true, // unknown conditions pass silently for forward compatibility
            };

            if !passed { return false; }
        }

        true
    }
}
