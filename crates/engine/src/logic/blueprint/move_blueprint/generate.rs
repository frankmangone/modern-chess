//! Working out the moves and threats a move blueprint gives from a position.

use std::collections::{HashMap, HashSet};

use super::{ActionBlueprint, Condition, Modifier, MoveBlueprint};
use crate::logic::blueprint::{
    conditions::{self, context::ConditionEvalContext},
    side_effects::{self, context::SideEffectContext},
};
use crate::logic::game::board_changes;
use crate::logic::{Board, Game, Piece};
use crate::shared::{BoardChange, Effect, EffectMetadata, Position, ALLY, EMPTY, ENEMY};

struct TargetState<'a> {
    target_position: Position,
    state: &'a str,
}

/// What stands on `target_position`, seen by `current_player`.
fn target_state<'a>(
    current_player: &str,
    target_position: Position,
    game: &'a Game,
) -> TargetState<'a> {
    let state = match game.piece_at_position_ref(&target_position) {
        None => EMPTY,
        Some(target_piece) if target_piece.player == current_player => ALLY,
        Some(_) => ENEMY,
    };
    TargetState {
        target_position,
        state,
    }
}

impl MoveBlueprint {
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

        // Threat-only blueprints contribute to the attack map but must not generate legal moves.
        if self.threat_only {
            return None;
        }

        if self.label_step {
            return self.calculate_linked_moves(piece, source_position, valid_move_ids, game);
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

            let Some(next_position) = next_position else {
                break;
            };
            let max_iterations_reached =
                !self.repeat_options.loop_move && iterations >= self.repeat_options.times;
            let until_condition_met =
                game.check_position_condition_str(&next_position, &self.repeat_options.until);

            if max_iterations_reached || until_condition_met {
                break;
            }

            current_source = next_position;
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
        let Some(target_state) = self.resolve_target_state(current_player, source_position, game)
        else {
            return (None, None);
        };
        let result_moves = self.judge_target(
            piece,
            original_source,
            source_position,
            &target_state,
            valid_move_ids,
            game,
        );
        (Some(result_moves), Some(target_state.target_position))
    }

    /// The moves of a label step on a map: one candidate for each place the label links the
    /// source to, each judged as a one-step move.
    fn calculate_linked_moves(
        &self,
        piece: &Piece,
        source_position: &Position,
        valid_move_ids: &HashSet<u8>,
        game: &Game,
    ) -> Option<Vec<(Position, Effect)>> {
        let current_player = game.current_player_ref();
        let step = self.step.get(current_player)?;
        let node = game.board.node(source_position)?;
        let mut all_moves: Vec<(Position, Effect)> = vec![];
        for &next in game.board.linked(node, step.label) {
            let Some(target_position) = game.board.name(next) else {
                continue;
            };
            let target_state = target_state(current_player, target_position.clone(), game);
            all_moves.extend(self.judge_target(
                piece,
                source_position,
                source_position,
                &target_state,
                valid_move_ids,
                game,
            ));
        }
        (!all_moves.is_empty()).then_some(all_moves)
    }

    /// The move to `target_state`, if its conditions, its action for what is there and that
    /// action's conditions allow it, with side effects, modifiers and post filters applied.
    fn judge_target(
        &self,
        piece: &Piece,
        original_source: &Position,
        source_position: &Position,
        target_state: &TargetState<'_>,
        valid_move_ids: &HashSet<u8>,
        game: &Game,
    ) -> Vec<(Position, Effect)> {
        let current_player = game.current_player_ref();
        let mut result_moves: Vec<(Position, Effect)> = Vec::new();
        let target_position = &target_state.target_position;

        // Check move-level conditions.
        let conditions_met = self.check_conditions(
            original_source,
            source_position,
            target_position,
            valid_move_ids,
            game,
        );
        if !conditions_met {
            return result_moves;
        }

        // Look up the action blueprint for the current board state.
        if let Some(action_bp) = self.actions.get(target_state.state) {
            let action_conds_met = self.action_conditions_met(
                action_bp,
                original_source,
                source_position,
                target_position,
                valid_move_ids,
                game,
            );
            if action_conds_met {
                let (moved_piece, extra_changes) = self.collect_side_effect_changes(
                    action_bp,
                    piece,
                    original_source,
                    source_position,
                    target_position,
                    current_player,
                    game,
                );
                let applied_modifier = self.resolve_modifier(target_position, game);
                let board_changes = self.build_board_changes(
                    original_source,
                    target_position,
                    moved_piece,
                    extra_changes,
                );
                let effect =
                    self.build_effect_for_action(action_bp, applied_modifier, board_changes);
                result_moves.push((target_position.clone(), effect));
            }
            // If action conditions fail: no move added, but position is still valid for looping.
        }

        self.apply_no_opponent_check_post_filter(&mut result_moves, current_player, game);

        result_moves
    }

    fn resolve_target_state<'a>(
        &self,
        current_player: &str,
        source_position: &Position,
        game: &'a Game,
    ) -> Option<TargetState<'a>> {
        // Follow the edge of this player's step.
        let step = self.step.get(current_player)?;
        let target_position = game.board.step(source_position, step.label)?.clone();
        Some(target_state(current_player, target_position, game))
    }

    fn action_conditions_met(
        &self,
        action_bp: &ActionBlueprint,
        original_source: &Position,
        source_position: &Position,
        target_position: &Position,
        valid_move_ids: &HashSet<u8>,
        game: &Game,
    ) -> bool {
        // Check action-level conditions (gates whether this specific action fires).
        self.evaluate_conditions(
            &action_bp.conditions,
            original_source,
            source_position,
            target_position,
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
                .all(|c| game.check_position_condition_str(target_position, c.kind.code()))
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
        if !game.rules.no_opponent_check {
            return;
        }

        result_moves.retain(|(_, effect)| {
            let sim = board_changes::simulate_from(&game.state.pieces, &effect.board_changes);
            game.rules
                .players
                .iter()
                .filter(|p| p.as_str() != current_player)
                .all(|opp| {
                    if game.rules.leader.is_empty() {
                        return true;
                    }
                    let leader_pos: Vec<Position> = sim
                        .iter()
                        .filter(|(_, p)| {
                            p.player == opp.as_str() && game.rules.leader.contains(&p.code)
                        })
                        .map(|(pos, _)| pos.clone())
                        .collect();
                    if leader_pos.is_empty() {
                        return true;
                    }
                    let my_attacks = crate::logic::Game::attacked_by_pieces(
                        current_player,
                        &sim,
                        &game.board,
                        &game.rules.blueprints,
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
        self.calculate_threats_with(player, source_position, &game.state.pieces, &game.board)
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
        let mut threats = HashSet::new();
        if !self.applies_to(player) || !self.actions.contains_key(ENEMY) {
            return threats;
        }
        let (Some(step), Some(mut node)) = (self.step.get(player), board.node(source_position))
        else {
            return threats;
        };

        // A label step threatens every linked place that an ally does not hold.
        if self.label_step {
            for &next in board.linked(node, step.label) {
                let Some(target_pos) = board.name(next) else {
                    continue;
                };
                if !matches!(pieces.get(target_pos), Some(p) if p.player == player) {
                    threats.insert(target_pos.clone());
                }
            }
            return threats;
        }

        let mut iterations: u8 = 1;
        loop {
            let Some(next) = board.follow(node, step.label) else {
                break;
            };
            let Some(target_pos) = board.name(next) else {
                break;
            };

            match pieces.get(target_pos) {
                Some(p) if p.player == player => break, // ally blocks
                Some(_) => {
                    threats.insert(target_pos.clone());
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

            node = next;
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
        target_position: &Position,
        valid_move_ids: &HashSet<u8>,
        game: &Game,
    ) -> bool {
        self.evaluate_conditions(
            &self.conditions,
            original_source,
            source_position,
            target_position,
            valid_move_ids,
            game,
        )
    }

    /// Core condition evaluator. Accepts any slice of conditions so it can be used for
    /// both move-level (`self.conditions`) and action-level (`action_bp.conditions`) checks.
    ///
    /// `original_source` is the piece's actual position on the board (constant across repeat
    /// iterations). `source_position` is the current loop position (may advance each iteration).
    /// `target_position` is where this step lands from `source_position`.
    fn evaluate_conditions(
        &self,
        conditions: &[Condition],
        original_source: &Position,
        source_position: &Position,
        target_position: &Position,
        valid_move_ids: &HashSet<u8>,
        game: &Game,
    ) -> bool {
        let ctx = ConditionEvalContext::new(
            self,
            original_source,
            source_position,
            target_position,
            valid_move_ids,
            game,
        );
        for condition in conditions {
            if !conditions::evaluate_condition(condition, &ctx) {
                return false;
            }
        }
        true
    }
}
