//! Working out the places and threats a move blueprint gives from a position.

use std::collections::{HashMap, HashSet};

use super::MoveBlueprint;
use crate::logic::blueprint::test_block::{self, TestContext};
use crate::logic::{Board, Game, Piece};
use crate::shared::Position;

impl MoveBlueprint {
    /// Whether this move is available to `player` (see `MoveSpec::players`).
    fn applies_to(&self, player: &str) -> bool {
        self.players.is_empty() || self.players.iter().any(|p| p == player)
    }

    /// Whether a repeat of `player`'s from `original_source` stops at `next_position`, the
    /// square just reached, judged on `pieces`.
    fn stops_at(
        &self,
        player: &str,
        original_source: &Position,
        next_position: &Position,
        pieces: &HashMap<Position, Piece>,
        game: &Game,
    ) -> bool {
        test_block::judge(
            &self.repeat_options.until,
            &TestContext {
                game,
                pieces,
                player,
                start: Some(original_source),
                target: next_position,
                each: None,
                removed: &[],
                moved: &[],
                after_turn: None,
            },
        )
    }

    /// The places this move reaches from `source` for `player`, in the order it reaches them,
    /// before any of its own steps run: its `step` followed once, or repeated until `until`
    /// holds (judged on `pieces`) or the repeat is spent, or every place a map's label links
    /// to. None for a move that is not `player`'s, or that only threatens.
    pub(crate) fn candidate_targets(
        &self,
        player: &str,
        source: &Position,
        pieces: &HashMap<Position, Piece>,
        game: &Game,
    ) -> Vec<Position> {
        let mut targets = Vec::new();
        if !self.applies_to(player) || self.threat_only {
            return targets;
        }
        let Some(step) = self.step.get(player) else {
            return targets;
        };
        let board = &game.board;
        if self.label_step {
            if let Some(node) = board.node(source) {
                let linked = board.linked(node, step.label).iter();
                targets.extend(linked.filter_map(|&next| board.name(next).cloned()));
            }
            return targets;
        }
        let mut current = source;
        let mut iterations: u8 = 1;
        while let Some(next) = board.step(current, step.label) {
            targets.push(*next);
            let spent = !self.repeat_options.loop_move && iterations >= self.repeat_options.times;
            if spent || self.stops_at(player, source, next, pieces, game) {
                break;
            }
            current = next;
            iterations = iterations.saturating_add(1);
        }
        targets
    }

    /// Returns the set of squares threatened by this move blueprint from `source_position`,
    /// ignoring move-level conditions (threat is about board geometry, not turn state).
    /// Only moves that `threatens` contribute to the attack map.
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
        if !self.applies_to(player) || !self.threatens {
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
                    threats.insert(*target_pos);
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
                    threats.insert(*target_pos);
                    break;
                } // enemy: threatened, stop
                None => {
                    threats.insert(*target_pos);
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
}
