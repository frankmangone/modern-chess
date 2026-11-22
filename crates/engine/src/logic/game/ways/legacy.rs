//! The ways of a spec written in the old forms, with no `actions`, synthesized from today's
//! move generator, drops, supplies and pass so that they offer exactly what today's turn flow
//! offers. It goes once every spec is written in actions.

use super::{Answer, ChoiceKind, Way};
use crate::logic::{Game, Piece};
use crate::shared::{BoardChange, EffectMetadata, Position, CAPTURE, DROP, MOVE, PASS, TRANSFORM};

impl Game {
    /// The ways of the player to move, before the rules that apply to every move:
    /// - `MOVE`: a piece's place and its target, from the move generator, with one way per
    ///   option of a promotion, which answers the option's code (unless the piece did not
    ///   survive its move);
    /// - `DROP`, where the hand is kept: a code in the hand and a square;
    /// - one action per code of an unlimited supply, named after it: a square;
    /// - `PASS`, where passing is allowed: no answers.
    pub(super) fn legacy_ways(&self) -> Vec<Way> {
        let player = self.current_player_ref();
        let mut ways = self.legacy_moves(player);

        if self.rules.hand_enabled {
            let mut held: Vec<&String> = self
                .state
                .hand
                .get(player)
                .into_iter()
                .flatten()
                .filter(|(_, count)| **count > 0)
                .map(|(code, _)| code)
                .collect();
            held.sort();
            for code in held {
                for (square, effect) in self.drop_candidates(code, player) {
                    ways.push(Way {
                        action: DROP.to_string(),
                        answers: vec![Answer::Hand(code.clone()), Answer::Place(square.clone())],
                        kinds: vec![ChoiceKind::Hand, ChoiceKind::Place],
                        changes: effect.board_changes,
                        from_hand: Some(code.clone()),
                        captures: false,
                        ends_turn: false,
                        lands: Some(square),
                        effects_before: Vec::new(),
                    });
                }
            }
        }

        let mut supplied: Vec<&String> = self.rules.unlimited_supply.iter().collect();
        supplied.sort();
        for code in supplied {
            for (square, effect) in self.drop_candidates(code, player) {
                ways.push(Way {
                    action: code.clone(),
                    answers: vec![Answer::Place(square)],
                    kinds: vec![ChoiceKind::Place],
                    changes: effect.board_changes,
                    from_hand: None,
                    captures: false,
                    ends_turn: false,
                    lands: None,
                    effects_before: Vec::new(),
                });
            }
        }

        if self.rules.allow_pass {
            ways.push(Way {
                action: PASS.to_string(),
                answers: Vec::new(),
                kinds: Vec::new(),
                changes: Vec::new(),
                from_hand: None,
                captures: false,
                ends_turn: false,
                lands: None,
                effects_before: Vec::new(),
            });
        }
        ways
    }

    /// The `MOVE` ways of `player`'s pieces, in board order and then target order.
    fn legacy_moves(&self, player: &str) -> Vec<Way> {
        let mut origins: Vec<(&Position, &Piece)> = self
            .state
            .pieces
            .iter()
            .filter(|(_, piece)| piece.player == player)
            .collect();
        origins.sort_by(|(a, _), (b, _)| a.cmp(b));

        let mut ways = Vec::new();
        for (from, piece) in origins {
            let Some(blueprint) = self.rules.blueprints.get(&piece.code) else {
                continue;
            };
            let mut moves: Vec<_> = blueprint
                .calculate_moves(piece, from, self)
                .unwrap_or_default()
                .into_iter()
                .collect();
            moves.sort_by(|(a, _), (b, _)| a.cmp(b));
            for (target, effect) in moves {
                let options = match effect.metadata {
                    Some(EffectMetadata::Options(options)) if effect.action == TRANSFORM => options,
                    _ => Vec::new(),
                };
                let way = Way {
                    action: MOVE.to_string(),
                    answers: vec![Answer::Place(from.clone()), Answer::Place(target.clone())],
                    kinds: vec![ChoiceKind::Piece, ChoiceKind::Move],
                    lands: self
                        .standing_after(&effect.board_changes, &target)
                        .filter(|standing| standing.player == player)
                        .map(|_| target.clone()),
                    changes: effect.board_changes,
                    from_hand: None,
                    captures: effect.action == CAPTURE,
                    ends_turn: effect.action == TRANSFORM,
                    effects_before: Vec::new(),
                };
                // Nothing to promote when the piece did not survive its move (an explosion
                // took it with its victim): the move asks nothing more, as today.
                let standing = self.standing_after(&way.changes, &target).is_some();
                if options.is_empty() || !standing {
                    ways.push(way);
                    continue;
                }
                for option in options {
                    let mut promoted = way.clone();
                    // The code is asked once the piece stands on its target.
                    promoted.effects_before.push((2, promoted.changes.len()));
                    promoted.answers.push(Answer::Code(option.clone()));
                    promoted.kinds.push(ChoiceKind::Code);
                    let piece = Piece::new(option, player.to_string());
                    promoted
                        .changes
                        .push(BoardChange::set_piece(target.clone(), piece));
                    ways.push(promoted);
                }
            }
        }
        ways
    }

    /// The piece on `at` once `changes` are made.
    fn standing_after<'a>(
        &'a self,
        changes: &'a [BoardChange],
        at: &Position,
    ) -> Option<&'a Piece> {
        match changes.iter().rev().find(|change| change.position == *at) {
            Some(change) => change.piece.as_ref(),
            None => self.state.pieces.get(at),
        }
    }
}
