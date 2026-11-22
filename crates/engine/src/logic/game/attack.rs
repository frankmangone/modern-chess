use std::collections::{HashMap, HashSet};

use crate::logic::blueprint::PieceBlueprint;
use crate::logic::game::board_changes;
use crate::logic::{Board, Game, Piece};
use crate::shared::Position;

impl Game {
    /// Returns the set of positions threatened by all pieces belonging to `attacker`.
    pub fn attacked_by(&self, attacker: &str) -> HashSet<Position> {
        Self::attacked_by_pieces(
            attacker,
            &self.state.pieces,
            &self.board,
            &self.rules.blueprints,
        )
    }

    /// Returns the player who acted just before the current turn.
    pub fn previous_player(&self) -> String {
        let len = self.state.turn_order.len();
        let Some(idx) = (self.state.current_turn as usize + len.saturating_sub(1)).checked_rem(len)
        else {
            return String::new();
        };
        self.state.turn_order.get(idx).cloned().unwrap_or_default()
    }

    /// Computes the attack set for `attacker` using an explicit pieces map (for simulation).
    pub(crate) fn attacked_by_pieces(
        attacker: &str,
        pieces: &HashMap<Position, Piece>,
        board: &Board,
        blueprints: &HashMap<String, PieceBlueprint>,
    ) -> HashSet<Position> {
        pieces
            .iter()
            .filter(|(_, p)| p.player == attacker)
            .filter_map(|(pos, p)| blueprints.get(&p.code).map(|bp| (pos, bp)))
            .flat_map(|(pos, bp)| bp.calculate_threats_with(attacker, pos, pieces, board))
            .collect()
    }

    /// Returns true if the current player's leader is in check given a simulated pieces map.
    /// A player is in check when *any* of their leader-coded pieces is attacked.
    pub(crate) fn leader_in_check_for_pieces(&self, pieces: &HashMap<Position, Piece>) -> bool {
        if self.rules.leader.is_empty() {
            return false;
        }
        let player = self.current_player_ref();
        let leader_positions: Vec<Position> = pieces
            .iter()
            .filter(|(_, p)| p.player == player && self.rules.leader.contains(&p.code))
            .map(|(pos, _)| pos.clone())
            .collect();
        if leader_positions.is_empty() {
            // No leader in this (possibly simulated) board.
            // Return true (= treat as in check / illegal) only if this player was
            // configured with a leader in their starting positions.  This prevents:
            //   • Moves that would explode/capture the own king from appearing legal.
            //   • The opponent's king destruction from being ignored by check_game_over.
            // Returns false for test setups or games that simply don't place a leader.
            return self.rules.players_with_configured_leader.contains(player);
        }
        self.rules
            .players
            .iter()
            .filter(|p| p.as_str() != player)
            .any(|opp| {
                pieces
                    .iter()
                    .filter(|(_, p)| p.player == *opp)
                    .any(|(from, attacker)| {
                        let Some(bp) = self.rules.blueprints.get(&attacker.code) else {
                            return false;
                        };
                        let threats = bp.calculate_threats_with(opp, from, pieces, &self.board);
                        leader_positions.iter().any(|target| {
                            threats.contains(target)
                                && self.capture_is_a_real_threat(from, attacker, target, pieces)
                        })
                    })
            })
    }

    /// Whether the piece at `from` could really take the leader at `target`.
    ///
    /// It always can, unless some move removes more than its target (captures explode): then
    /// taking a leader that stands next to the capturer's own leader would destroy both, which
    /// is not allowed, so it is no threat. This is why two kings may stand side by side in
    /// Atomic Chess. Only those games pay for the simulation, which copies the game.
    fn capture_is_a_real_threat(
        &self,
        from: &Position,
        attacker: &Piece,
        target: &Position,
        pieces: &HashMap<Position, Piece>,
    ) -> bool {
        let owns_a_leader = |pieces: &HashMap<Position, Piece>| {
            pieces
                .values()
                .any(|p| p.player == attacker.player && self.rules.leader.contains(&p.code))
        };
        if !self.rules.removes_beyond_target || !owns_a_leader(pieces) {
            return true;
        }
        // The moves are worked out on the simulated board with the attacker to move, so the
        // blast is too. Without that, the leader would count as the mover's own piece.
        let Some(turn) = self
            .state
            .turn_order
            .iter()
            .position(|p| *p == attacker.player)
        else {
            return true;
        };
        let mut simulated = self.clone();
        simulated.state.pieces = pieces.clone();
        simulated.state.current_turn = turn as u8;
        // The capture as the attacker's moves make it: the first way to `target` in spec order.
        let changes = simulated
            .piece_move_ways(from, target)
            .into_iter()
            .next()
            .map(|way| way.changes);
        let Some(changes) = changes else {
            return true;
        };
        let after = board_changes::simulate_from(pieces, &changes);
        owns_a_leader(&after)
    }

    /// Returns true if the current player's leader is currently in check.
    pub fn leader_in_check(&self) -> bool {
        self.leader_in_check_for_pieces(&self.state.pieces)
    }
}
