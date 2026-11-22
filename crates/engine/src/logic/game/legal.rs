use std::collections::HashMap;

use crate::logic::game::board_changes;
use crate::logic::{Game, Piece, Way};
use crate::shared::{BoardChange, Position};

impl Game {
    /// Whether the player to move has at least one way of taking an action, worked out from
    /// the board as it stands. A game with no ways has no legal move.
    pub fn any_legal_moves(&self) -> bool {
        !self.compute_ways().is_empty()
    }

    /// Whether the current player may play a move that leaves the board as `after`.
    ///
    /// - Never when it destroys their own leader (an Atomic capture next to their own king).
    /// - Always when it destroys an opponent's leader: the game ends on the spot, so being in
    ///   check no longer matters.
    /// - Otherwise only when their leader is not attacked.
    pub(crate) fn move_is_safe(&self, after: &HashMap<Position, Piece>) -> bool {
        if self.rules.leader.is_empty() {
            return true;
        }
        let player = self.current_player_ref();
        let has_leader = |pieces: &HashMap<Position, Piece>, owner: &str| {
            pieces
                .values()
                .any(|p| p.player == owner && self.rules.leader.contains(&p.code))
        };
        if self.rules.players_with_configured_leader.contains(player) && !has_leader(after, player)
        {
            return false;
        }
        let takes_a_leader = self
            .state
            .turn_order
            .iter()
            .filter(|other| other.as_str() != player)
            .any(|other| {
                self.rules
                    .players_with_configured_leader
                    .contains(other.as_str())
                    && has_leader(&self.state.pieces, other)
                    && !has_leader(after, other)
            });
        takes_a_leader || !self.leader_in_check_for_pieces(after)
    }

    /// Removes the ways that `move_is_safe` refuses. Does nothing when the player has no
    /// leader on the board (e.g. hand-built test positions).
    pub(crate) fn retain_leader_safe(&self, ways: &mut Vec<Way>) {
        if self.has_own_leader() {
            ways.retain(|way| self.leaves_leader_safe(&way.changes));
        }
    }

    /// Whether the player to move has a leader on the board.
    fn has_own_leader(&self) -> bool {
        let player = self.current_player_ref();
        self.state
            .pieces
            .values()
            .any(|p| p.player == player && self.rules.leader.contains(&p.code))
    }

    /// Whether `move_is_safe` allows the board as `changes` leave it.
    fn leaves_leader_safe(&self, changes: &[BoardChange]) -> bool {
        self.move_is_safe(&board_changes::simulate_from(&self.state.pieces, changes))
    }

    /// Under `no_opponent_check`, removes the ways that leave an opponent's leader attacked.
    pub(crate) fn retain_no_opponent_check(&self, ways: &mut Vec<Way>) {
        if self.rules.no_opponent_check {
            ways.retain(|way| !self.checks_an_opponent(&way.changes));
        }
    }

    /// Whether the board as `changes` leave it has a leader of an opponent of the player to
    /// move attacked by that player.
    pub(crate) fn checks_an_opponent(&self, changes: &[BoardChange]) -> bool {
        if self.rules.leader.is_empty() {
            return false;
        }
        let player = self.current_player_ref();
        let after = board_changes::simulate_from(&self.state.pieces, changes);
        let leaders: Vec<&Position> = after
            .iter()
            .filter(|(_, p)| p.player != player && self.rules.leader.contains(&p.code))
            .map(|(position, _)| position)
            .collect();
        if leaders.is_empty() {
            return false;
        }
        let attacks = Game::attacked_by_pieces(player, &after, &self.board, &self.rules.blueprints);
        leaders.iter().any(|leader| attacks.contains(*leader))
    }
}
