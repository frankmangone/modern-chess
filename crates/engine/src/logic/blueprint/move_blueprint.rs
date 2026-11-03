use std::collections::{HashMap, HashSet};

use super::{
    conditions::{self, context::ConditionEvalContext},
    side_effects::{self, context::SideEffectContext},
};
use crate::logic::game::board_changes;
use crate::logic::{Board, Game, Piece};
use crate::shared::{
    apply_direction, into_position, BoardChange, Effect, EffectMetadata, ExtendedPosition,
    Position, ALLY, EMPTY, ENEMY, NOT_EMPTY,
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

    // For PIECE_AT / PIECE_NOT_AT / ALLY_ADJACENT_COUNT: piece code to check.
    pub piece_code: Option<String>,

    // For PATH_PIECE_COUNT / ALLY_ADJACENT_COUNT: inclusive lower bound (default 0).
    pub min: Option<u8>,

    // For PATH_PIECE_COUNT / ALLY_ADJACENT_COUNT: inclusive upper bound (default u8::MAX).
    pub max: Option<u8>,

    // For PIECE_FIRST_MOVE: pass when target square is off-board or empty.
    pub absent_ok: bool,

    // For PIECE_AT: player relationship to check ("SELF", "ENEMY", "ANY").
    pub player: Option<String>,
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
    /// Also used by CONVERT as the position of the enemy to convert.
    pub target: Option<HashMap<String, ExtendedPosition>>,
    /// For CONVERT: the piece code to place at the target square (belonging to the acting player).
    /// If absent, defaults to the acting piece's own code.
    pub piece: Option<String>,
    /// For EXPLODE: blast radius in squares (default 1).
    pub radius: Option<u8>,
    /// For EXPLODE: piece codes exempt from the blast on adjacent squares.
    pub exempt_codes: Vec<String>,
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

    // Players this move is available to. Empty means every player.
    pub players: Vec<String>,
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

struct TargetState<'a> {
    target_position: Position,
    state: &'a str,
}

impl MoveBlueprint {
    pub fn from_spec(spec: MoveSpec, players_spec: Vec<PlayerSpec>) -> Self {
        // Helper: transform a raw [i8; 2] relative position for all players.
        fn transform_pos(
            raw: Option<[i8; 2]>,
            players: &[PlayerSpec],
        ) -> Option<HashMap<String, ExtendedPosition>> {
            raw.map(|r| {
                players
                    .iter()
                    .map(|p| {
                        let vec: ExtendedPosition = vec![r[0] as i16, r[1] as i16];
                        (p.name.clone(), apply_direction(&p.direction, &vec))
                    })
                    .collect()
            })
        }

        // Helper: convert a ConditionSpec slice into Condition structs, transforming
        // any relative positions per player.
        fn build_conditions(
            specs: &[crate::specs::game::piece::condition::ConditionSpec],
            players: &[PlayerSpec],
        ) -> Vec<Condition> {
            specs
                .iter()
                .map(|c| {
                    let position = c.position.map(|rel| {
                        players
                            .iter()
                            .map(|p| {
                                let canon = vec![rel[0] as i16, rel[1] as i16];
                                (p.name.clone(), apply_direction(&p.direction, &canon))
                            })
                            .collect()
                    });
                    Condition {
                        code: c.condition.clone(),
                        move_id: c.move_id,
                        state: c.state.clone(),
                        position,
                        piece_code: c.piece.clone(),
                        min: c.min,
                        max: c.max,
                        absent_ok: c.absent_ok,
                        player: c.player.clone(),
                    }
                })
                .collect()
        }

        // Build actions map: state -> ActionBlueprint.
        let mut actions = HashMap::new();
        for action_spec in spec.actions {
            let action_conditions = build_conditions(&action_spec.conditions, &players_spec);
            let action_side_effects: Vec<SideEffectBlueprint> = action_spec
                .side_effects
                .iter()
                .map(|se| SideEffectBlueprint {
                    action: se.action.clone(),
                    state: se.state.clone(),
                    duration: se.duration,
                    from: transform_pos(se.from, &players_spec),
                    to: transform_pos(se.to, &players_spec),
                    target: transform_pos(se.target, &players_spec),
                    piece: se.piece.clone(),
                    radius: se.radius,
                    exempt_codes: se.exempt_codes.clone(),
                })
                .collect();
            actions.insert(
                action_spec.state,
                ActionBlueprint {
                    action: action_spec.action,
                    conditions: action_conditions,
                    side_effects: action_side_effects,
                },
            );
        }

        // Transform each piece's canonical step by the player's direction matrix so that
        // "forward" in the spec maps to the correct board direction for every player.
        let mut step: HashMap<String, ExtendedPosition> = HashMap::new();
        for player_spec in &players_spec {
            let player_step = apply_direction(&player_spec.direction, &spec.step);
            step.insert(player_spec.name.clone(), player_step);
        }

        // Process repeat information.
        let (until, times, loop_move) = match spec.repeat {
            Some(repeat) => (
                repeat.until.unwrap_or_else(|| NOT_EMPTY.to_string()),
                repeat.times.unwrap_or(1u8),
                repeat.loop_move,
            ),
            None => (NOT_EMPTY.to_string(), 1u8, false),
        };

        // Process move-level conditions.
        let conditions = build_conditions(&spec.conditions, &players_spec);

        // Process modifiers.
        let modifiers = spec
            .modifiers
            .iter()
            .map(|m| Modifier {
                action: m.action.clone(),
                conditions: m
                    .conditions
                    .iter()
                    .map(|c| Condition {
                        code: c.condition.clone(),
                        move_id: None, // modifier conditions never use DEPENDS_ON
                        state: c.state.clone(),
                        position: None,
                        piece_code: c.piece.clone(),
                        min: c.min,
                        max: c.max,
                        absent_ok: c.absent_ok,
                        player: None,
                    })
                    .collect(),
                options: m.options.clone(),
            })
            .collect();

        // Process move-level side effects.
        let side_effects: Vec<SideEffectBlueprint> = spec
            .side_effects
            .iter()
            .map(|se| SideEffectBlueprint {
                action: se.action.clone(),
                state: se.state.clone(),
                duration: se.duration,
                from: transform_pos(se.from, &players_spec),
                to: transform_pos(se.to, &players_spec),
                target: transform_pos(se.target, &players_spec),
                piece: se.piece.clone(),
                radius: se.radius,
                exempt_codes: se.exempt_codes.clone(),
            })
            .collect();

        MoveBlueprint {
            id: spec.id,
            step,
            players: spec.players,
            actions,
            conditions,
            modifiers,
            side_effects,
            repeat_options: MoveRepeat {
                until,
                times,
                loop_move,
            },
        }
    }

    /// Whether this move is available to `player` (see `MoveSpec::players`).
    fn applies_to(&self, player: &str) -> bool {
        self.players.is_empty() || self.players.iter().any(|p| p == player)
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
        if !self.applies_to(&piece.player) {
            return None;
        }
        let original_source = source_position.clone();
        let mut iterations: u8 = 1;
        let mut current_source = source_position.clone();
        let mut all_moves: Vec<(Position, Effect)> = vec![];

        loop {
            let (moves, next_position) = self.calculate_single_move(
                piece,
                &original_source,
                &current_source,
                valid_move_ids,
                game,
            );

            if let Some(moves) = moves {
                all_moves.extend(moves);
            }

            let invalid_next_position = next_position.is_none();
            let max_iterations_reached =
                !self.repeat_options.loop_move && iterations >= self.repeat_options.times;
            let until_condition_met = match &next_position {
                Some(pos) => game.check_position_condition_str(pos, &self.repeat_options.until),
                None => false,
            };

            if invalid_next_position || max_iterations_reached || until_condition_met {
                break;
            }

            current_source = next_position.unwrap();
            iterations += 1u8;
        }

        (!all_moves.is_empty()).then_some(all_moves)
    }

    /// Calculates a single move based on a spec, and a board state. Used for recursive moves.
    /// First return value are the moves for this evaluation, second is the position to recurse to.
    /// If the latter is `None`, then the move is not repeatable.
    /// `original_source` is the piece's actual position (constant across all repeat iterations);
    /// `source_position` is the current loop position (advances each iteration).
    pub fn calculate_single_move(
        &self,
        piece: &Piece,
        original_source: &Position,
        source_position: &Position,
        valid_move_ids: &HashSet<u8>,
        game: &Game,
    ) -> (Option<Vec<(Position, Effect)>>, Option<Position>) {
        let current_player = game.current_player_ref();
        let mut result_moves: Vec<(Position, Effect)> = Vec::new();
        let Some(target_state) = self.resolve_target_state(current_player, source_position, game)
        else {
            return (None, None);
        };

        // Check move-level conditions.
        let conditions_met =
            self.check_conditions(original_source, source_position, valid_move_ids, game);
        if !conditions_met {
            return (Some(result_moves), Some(target_state.target_position));
        }

        // Look up the action blueprint for the current board state.
        if let Some(action_bp) = self.actions.get(target_state.state) {
            let action_conds_met = self.action_conditions_met(
                action_bp,
                original_source,
                source_position,
                valid_move_ids,
                game,
            );
            if action_conds_met {
                let (moved_piece, extra_changes) = self.collect_side_effect_changes(
                    action_bp,
                    piece,
                    original_source,
                    source_position,
                    &target_state.target_position,
                    current_player,
                    game,
                );
                let applied_modifier = self.resolve_modifier(&target_state.target_position, game);
                let board_changes = self.build_board_changes(
                    original_source,
                    &target_state.target_position,
                    moved_piece,
                    extra_changes,
                );
                let effect =
                    self.build_effect_for_action(action_bp, applied_modifier, board_changes);
                result_moves.push((target_state.target_position.clone(), effect));
            }
            // If action conditions fail: no move added, but position is still valid for looping.
        }

        self.apply_no_opponent_check_post_filter(&mut result_moves, current_player, game);

        (Some(result_moves), Some(target_state.target_position))
    }

    fn resolve_target_state<'a>(
        &self,
        current_player: &str,
        source_position: &Position,
        game: &'a Game,
    ) -> Option<TargetState<'a>> {
        // Component-wise addition of step (already transformed for this player).
        let target_position: Vec<i16> = source_position
            .iter()
            .zip(self.step.get(current_player).unwrap().iter())
            .map(|(&source, &step)| source as i16 + step)
            .collect();

        if !game.board.is_position_valid(&target_position) {
            return None;
        }

        let target_position = into_position(&target_position);
        let state = match game.piece_at_position_ref(&target_position) {
            None => EMPTY,
            Some(target_piece) if target_piece.player == current_player => ALLY,
            Some(_) => ENEMY,
        };

        Some(TargetState {
            target_position,
            state,
        })
    }

    fn action_conditions_met(
        &self,
        action_bp: &ActionBlueprint,
        original_source: &Position,
        source_position: &Position,
        valid_move_ids: &HashSet<u8>,
        game: &Game,
    ) -> bool {
        // Check action-level conditions (gates whether this specific action fires).
        self.evaluate_conditions(
            &action_bp.conditions,
            original_source,
            source_position,
            valid_move_ids,
            game,
        )
    }

    fn collect_side_effect_changes(
        &self,
        action_bp: &ActionBlueprint,
        piece: &Piece,
        original_source: &Position,
        source_position: &Position,
        target_position: &Position,
        current_player: &str,
        game: &Game,
    ) -> (Piece, Vec<BoardChange>) {
        let mut moved_piece = piece.clone();
        moved_piece.total_moves += 1u16;

        // Move-level side effects always fire; action-level side effects only when
        // this action fires.
        let mut extra_changes: Vec<BoardChange> = Vec::new();
        let side_effect_context = SideEffectContext::new(
            game,
            current_player,
            piece,
            original_source,
            source_position,
            target_position,
        );
        for se in self
            .side_effects
            .iter()
            .chain(action_bp.side_effects.iter())
        {
            side_effects::apply_side_effect(
                se,
                &side_effect_context,
                &mut moved_piece,
                &mut extra_changes,
            );
        }

        (moved_piece, extra_changes)
    }

    fn resolve_modifier(&self, target_position: &Position, game: &Game) -> Option<Modifier> {
        // Check for a modifier (e.g. pawn promotion).
        for modifier in &self.modifiers {
            if modifier
                .conditions
                .iter()
                .all(|c| game.check_position_condition_str(target_position, &c.code))
            {
                return Some(modifier.clone());
            }
        }
        None
    }

    fn build_board_changes(
        &self,
        original_source: &Position,
        target_position: &Position,
        moved_piece: Piece,
        extra_changes: Vec<BoardChange>,
    ) -> Vec<BoardChange> {
        let mut board_changes = vec![
            BoardChange::clear(original_source),
            BoardChange::set_piece(target_position.clone(), moved_piece),
        ];
        board_changes.extend(extra_changes);
        board_changes
    }

    fn build_effect_for_action(
        &self,
        action_bp: &ActionBlueprint,
        applied_modifier: Option<Modifier>,
        board_changes: Vec<BoardChange>,
    ) -> Effect {
        match applied_modifier {
            Some(modifier) => Effect {
                action: modifier.action,
                board_changes,
                metadata: Some(EffectMetadata::Options(modifier.options)),
            },
            None => Effect {
                action: action_bp.action.clone(),
                board_changes,
                metadata: None,
            },
        }
    }

    fn apply_no_opponent_check_post_filter(
        &self,
        result_moves: &mut Vec<(Position, Effect)>,
        current_player: &str,
        game: &Game,
    ) {
        // no_opponent_check post-filter: after simulating each effect's board changes,
        // verify that no opponent's leader is attacked by the current player.
        if !game.no_opponent_check {
            return;
        }

        result_moves.retain(|(_, effect)| {
            let sim = board_changes::simulate_from(&game.state.pieces, &effect.board_changes);
            game.players
                .iter()
                .filter(|p| p.as_str() != current_player)
                .all(|opp| {
                    if game.leader.is_empty() {
                        return true;
                    }
                    let leader_pos: Vec<Position> = sim
                        .iter()
                        .filter(|(_, p)| p.player == opp.as_str() && game.leader.contains(&p.code))
                        .map(|(pos, _)| pos.clone())
                        .collect();
                    if leader_pos.is_empty() {
                        return true;
                    }
                    let my_attacks = crate::logic::Game::attacked_by_pieces(
                        current_player,
                        &sim,
                        &game.board,
                        &game.blueprints,
                    );
                    leader_pos.iter().all(|lp| !my_attacks.contains(lp))
                })
        });
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
        if !self.applies_to(player) || !self.actions.contains_key(ENEMY) {
            return HashSet::new();
        }

        let mut threats = HashSet::new();
        let mut iterations: u8 = 1;
        let mut current_source = source_position.clone();

        loop {
            let Some(step) = self.step.get(player) else {
                break;
            };
            let target: Vec<i16> = current_source
                .iter()
                .zip(step.iter())
                .map(|(&s, &st)| s as i16 + st)
                .collect();

            if !game.board.is_position_valid(&target) {
                break;
            }
            let target_pos = into_position(&target);

            match game.piece_at_position_ref(&target_pos) {
                Some(p) if p.player == player => break, // ally blocks
                Some(_) => {
                    threats.insert(target_pos);
                    break;
                } // enemy: threatened, stop
                None => {
                    threats.insert(target_pos.clone());
                } // empty: threatened, continue
            }

            let max_iterations_reached =
                !self.repeat_options.loop_move && iterations >= self.repeat_options.times;
            if max_iterations_reached {
                break;
            }

            current_source = target_pos;
            iterations += 1;
        }

        threats
    }

    /// Like `calculate_threats`, but uses an explicit `pieces` map and `board` instead of
    /// `game.state.pieces` / `game.board`. Used for simulated check detection.
    pub fn calculate_threats_with(
        &self,
        player: &str,
        source_position: &Position,
        pieces: &HashMap<Position, Piece>,
        board: &Board,
    ) -> HashSet<Position> {
        if !self.applies_to(player) || !self.actions.contains_key(ENEMY) {
            return HashSet::new();
        }

        let mut threats = HashSet::new();
        let mut iterations: u8 = 1;
        let mut current_source = source_position.clone();

        loop {
            let Some(step) = self.step.get(player) else {
                break;
            };
            let target: Vec<i16> = current_source
                .iter()
                .zip(step.iter())
                .map(|(&s, &st)| s as i16 + st)
                .collect();

            if !board.is_position_valid(&target) {
                break;
            }
            let target_pos = into_position(&target);

            match pieces.get(&target_pos) {
                Some(p) if p.player == player => break,
                Some(_) => {
                    threats.insert(target_pos);
                    break;
                }
                None => {
                    threats.insert(target_pos.clone());
                }
            }

            let max_iterations_reached =
                !self.repeat_options.loop_move && iterations >= self.repeat_options.times;
            if max_iterations_reached {
                break;
            }

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
        original_source: &Position,
        source_position: &Position,
        valid_move_ids: &HashSet<u8>,
        game: &Game,
    ) -> bool {
        self.evaluate_conditions(
            &self.conditions,
            original_source,
            source_position,
            valid_move_ids,
            game,
        )
    }

    /// Core condition evaluator. Accepts any slice of conditions so it can be used for
    /// both move-level (`self.conditions`) and action-level (`action_bp.conditions`) checks.
    ///
    /// `original_source` is the piece's actual position on the board (constant across repeat
    /// iterations). `source_position` is the current loop position (may advance each iteration).
    fn evaluate_conditions(
        &self,
        conditions: &[Condition],
        original_source: &Position,
        source_position: &Position,
        valid_move_ids: &HashSet<u8>,
        game: &Game,
    ) -> bool {
        let ctx =
            ConditionEvalContext::new(self, original_source, source_position, valid_move_ids, game);
        for condition in conditions {
            if !conditions::evaluate_condition(condition, &ctx) {
                return false;
            }
        }
        true
    }
}
