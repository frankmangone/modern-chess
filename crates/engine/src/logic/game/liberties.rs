use std::collections::HashSet;

use crate::logic::game::board_changes;
use crate::logic::{Game, Piece};
use crate::shared::{into_extended_position, into_position, BoardChange, Effect, Position};
use crate::specs::LibertyCaptureSpec;

use std::collections::HashMap;

/// What a move does to the groups next to the squares it touches.
struct Settlement {
    /// Stones that leave the board: the opponents' groups that were shut in, and the mover's own
    /// when suicide is allowed.
    removed: Vec<Position>,
    /// The mover left one of their own groups with no liberties, and it was not saved by a capture.
    suicide: bool,
}

impl Game {
    /// The squares next to `position` along a row or a column that are on the board.
    pub(crate) fn orthogonal_neighbours(&self, position: &Position) -> Vec<Position> {
        let mut found = Vec::new();
        for axis in 0..position.len() {
            for delta in [-1i16, 1] {
                let mut candidate = into_extended_position(position);
                if let Some(value) = candidate.get_mut(axis) {
                    *value += delta;
                }
                if self.board.is_position_valid(&candidate) {
                    found.push(into_position(&candidate));
                }
            }
        }
        found
    }

    /// Completes a candidate move with the rules that look at the whole board, in this order:
    ///
    /// 1. Liberty capture: opponents' groups with no liberties are removed (and the mover's own,
    ///    when suicide is allowed). A move that would leave the mover's own group with no
    ///    liberties is refused when it is not allowed.
    /// 2. No repeated positions: a move that recreates an earlier position is refused.
    ///
    /// Returns `None` for a refused move, otherwise the effect with the removals added.
    pub(crate) fn settle(&self, mut effect: Effect) -> Option<Effect> {
        if let Some(rule) = &self.liberty_capture {
            let after = board_changes::simulate_from(&self.state.pieces, &effect.board_changes);
            let touched: Vec<&Position> =
                effect.board_changes.iter().map(|c| &c.position).collect();
            let settlement =
                self.settle_liberties(rule, &after, self.current_player_ref(), &touched);
            if settlement.suicide && !rule.allow_suicide {
                return None;
            }
            effect
                .board_changes
                .extend(settlement.removed.iter().map(BoardChange::clear));
        }

        if self.forbid_repeated_positions && self.recreates_a_position(&effect) {
            return None;
        }
        Some(effect)
    }

    /// Whether playing `effect` would leave a position that has already occurred.
    fn recreates_a_position(&self, effect: &Effect) -> bool {
        if self.state.position_hashes.is_empty() {
            return false;
        }
        let mut after = board_changes::simulate_from(&self.state.pieces, &effect.board_changes);
        // The turn passing ticks every piece's timed flags before the position is recorded.
        for piece in after.values_mut() {
            piece.tick_state_flags();
        }
        let key = self.position_key_for(&after, self.player_after_current());
        self.state.position_hashes.contains(&key)
    }

    /// The groups of `rule.pieces` that touch (or contain) one of the `touched` squares, and
    /// what becomes of them once the mover has played.
    fn settle_liberties(
        &self,
        rule: &LibertyCaptureSpec,
        pieces: &HashMap<Position, Piece>,
        mover: &str,
        touched: &[&Position],
    ) -> Settlement {
        let mut seeds: Vec<Position> = Vec::new();
        for square in touched {
            seeds.push((*square).clone());
            seeds.extend(self.orthogonal_neighbours(square));
        }

        // First the opponents' groups, judged on the board as the mover left it.
        let mut removed: HashSet<Position> = HashSet::new();
        let mut seen: HashSet<Position> = HashSet::new();
        for seed in &seeds {
            if seen.contains(seed) {
                continue;
            }
            let Some(group) = self.group_at(rule, pieces, &removed, seed) else {
                continue;
            };
            seen.extend(group.members.iter().cloned());
            if !group.has_liberty && group.player != mover {
                removed.extend(group.members);
            }
        }

        // Then the mover's own groups, with the points just freed counted as empty.
        let mut suicide = false;
        let mut own_removed: Vec<Position> = Vec::new();
        let mut seen: HashSet<Position> = HashSet::new();
        for seed in &seeds {
            if seen.contains(seed) {
                continue;
            }
            let Some(group) = self.group_at(rule, pieces, &removed, seed) else {
                continue;
            };
            seen.extend(group.members.iter().cloned());
            if !group.has_liberty && group.player == mover {
                suicide = true;
                own_removed.extend(group.members);
            }
        }

        let mut all: Vec<Position> = removed.into_iter().collect();
        if rule.allow_suicide {
            all.extend(own_removed);
        }
        all.sort();
        Settlement {
            removed: all,
            suicide,
        }
    }

    /// The group of `rule.pieces` that contains `start`: its stones, its owner, and whether it
    /// has a liberty. Squares in `removed` count as empty. `None` when `start` holds no such piece.
    fn group_at(
        &self,
        rule: &LibertyCaptureSpec,
        pieces: &HashMap<Position, Piece>,
        removed: &HashSet<Position>,
        start: &Position,
    ) -> Option<Group> {
        let standing = |position: &Position| {
            pieces
                .get(position)
                .filter(|piece| !removed.contains(position) && rule.pieces.contains(&piece.code))
        };
        let player = standing(start)?.player.clone();

        let mut members: HashSet<Position> = HashSet::new();
        let mut has_liberty = false;
        let mut stack = vec![start.clone()];
        while let Some(position) = stack.pop() {
            if !members.insert(position.clone()) {
                continue;
            }
            for next in self.orthogonal_neighbours(&position) {
                if members.contains(&next) {
                    continue;
                }
                match standing(&next) {
                    Some(piece) if piece.player == player => stack.push(next),
                    Some(_) => {}
                    None => {
                        // Empty, or just emptied by a capture. A piece of some other kind that is
                        // not part of the rule blocks the way instead.
                        let blocked = pieces.contains_key(&next) && !removed.contains(&next);
                        if !blocked {
                            has_liberty = true;
                        }
                    }
                }
            }
        }

        Some(Group {
            members: members.into_iter().collect(),
            player,
            has_liberty,
        })
    }
}

struct Group {
    members: Vec<Position>,
    player: String,
    has_liberty: bool,
}
