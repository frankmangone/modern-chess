//! Working out the places and threats a move blueprint gives from a position.

use crate::logic::PiecesView;
use crate::shared::IdSet;

use super::MoveBlueprint;
use crate::logic::rules::test_block::{self, TestContext};
use crate::logic::{Game, Piece};
use crate::shared::{Position, Symbol};

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
        pieces: PiecesView<'_>,
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
                last: None,
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
        pieces: PiecesView<'_>,
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

    /// The squares this move attacks from `source`: those it could take on, by its own rules
    /// (see `visit_threats`). `game` gives the rules and the board; `pieces` the position.
    pub fn calculate_threats_with(
        &self,
        game: &Game,
        player: &str,
        source: &Position,
        pieces: PiecesView<'_>,
    ) -> IdSet<Position> {
        let mut threats = IdSet::default();
        self.visit_threats(game, player, source, pieces, None, &mut |threat| {
            threats.insert(*threat);
            false
        });
        threats
    }

    /// Whether the move's conditions hold for `player`'s piece at `source` going to `target`,
    /// with the piece that stands there, or, on an empty square, a stand-in `ghost`: a piece of
    /// another player with no code, so that the move is asked whether it could take there.
    fn admits(
        &self,
        game: &Game,
        player: &str,
        source: &Position,
        target: &Position,
        pieces: PiecesView<'_>,
    ) -> bool {
        let Some(conditions) = self
            .conditions
            .get(player)
            .filter(|tests| !tests.is_empty())
        else {
            return true;
        };
        let ghost;
        let view = if pieces.get(target).is_some() {
            pieces
        } else {
            let owner = game
                .rules
                .players
                .iter()
                .find(|name| name.as_str() != player)
                .map_or("", |name| name.as_str());
            ghost = Piece::new(Symbol::new(""), Symbol::new(owner));
            pieces.holding(target, &ghost)
        };
        let ctx = TestContext {
            game,
            pieces: view,
            player,
            start: Some(source),
            target,
            each: None,
            removed: &[],
            moved: &[],
            after_turn: None,
            last: None,
        };
        conditions.iter().all(|test| test_block::judge(test, &ctx))
    }

    /// Calls `found` with each square this move attacks from `source`, in the order it reaches
    /// them, and stops at the first for which `found` returns true, which is then the result.
    /// No set is built.
    ///
    /// A piece attacks a square if this move could take there: the squares are the ones move
    /// generation reaches (the step, the repeat and its `until`), and the move's conditions, the
    /// tests at the start of its steps, must hold with a piece of another player on the square
    /// (the one that is there, or a ghost). The rules that apply to every action are not applied,
    /// and a condition that reads the way in progress or the attack map holds. With `wanted`,
    /// only those squares are judged and offered.
    pub(crate) fn visit_threats(
        &self,
        game: &Game,
        player: &str,
        source: &Position,
        pieces: PiecesView<'_>,
        wanted: Option<&[&Position]>,
        found: &mut impl FnMut(&Position) -> bool,
    ) -> bool {
        if !self.applies_to(player) || !self.threatens {
            return false;
        }
        let (Some(step), Some(mut node)) = (self.step.get(player), game.board.node(source)) else {
            return false;
        };
        let board = &game.board;
        let mut reaches = |target: &Position| {
            wanted.is_none_or(|places| places.contains(&target))
                && self.admits(game, player, source, target, pieces)
                && found(target)
        };

        // A label step reaches every place it links to.
        if self.label_step {
            for &next in board.linked(node, step.label) {
                let Some(target) = board.name(next) else {
                    continue;
                };
                if reaches(target) {
                    return true;
                }
            }
            return false;
        }

        let mut iterations: u8 = 1;
        while let Some(next) = board.follow(node, step.label) {
            let Some(target) = board.name(next) else {
                break;
            };
            if reaches(target) {
                return true;
            }
            let spent = !self.repeat_options.loop_move && iterations >= self.repeat_options.times;
            let stops = if self.repeat_options.default_until {
                pieces.get(target).is_some()
            } else {
                self.stops_at(player, source, target, pieces, game)
            };
            if spent || stops {
                break;
            }
            node = next;
            iterations = iterations.saturating_add(1);
        }
        false
    }
}
