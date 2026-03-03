use std::collections::{HashMap, HashSet};

use crate::logic::blueprint::PieceBlueprint;
use crate::logic::{Board, Game, Piece};
use crate::shared::Position;

impl Game {
    /// Returns the set of positions threatened by all pieces belonging to `attacker`.
    pub fn attacked_by(&self, attacker: &str) -> HashSet<Position> {
        Self::attacked_by_pieces(attacker, &self.state.pieces, &self.board, &self.blueprints)
    }

    /// Returns the player who acted just before the current turn.
    pub fn previous_player(&self) -> String {
        let len = self.turn_order.len() as u8;
        let idx = (self.state.current_turn + len - 1) % len;
        self.turn_order[idx as usize].clone()
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
        if self.leader.is_empty() {
            return false;
        }
        let player = self.current_player_ref();
        let leader_positions: Vec<Position> = pieces
            .iter()
            .filter(|(_, p)| p.player == player && self.leader.contains(&p.code))
            .map(|(pos, _)| pos.clone())
            .collect();
        if leader_positions.is_empty() {
            // No leader in this (possibly simulated) board.
            // Return true (= treat as in check / illegal) only if this player was
            // configured with a leader in their starting positions.  This prevents:
            //   • Moves that would explode/capture the own king from appearing legal.
            //   • The opponent's king destruction from being ignored by check_game_over.
            // Returns false for test setups or games that simply don't place a leader.
            return self.players_with_configured_leader.contains(player);
        }
        self.players
            .iter()
            .filter(|p| p.as_str() != player)
            .any(|opp| {
                let attacks = Self::attacked_by_pieces(opp, pieces, &self.board, &self.blueprints);
                leader_positions.iter().any(|pos| attacks.contains(pos))
            })
    }

    /// Returns true if the current player's leader is currently in check.
    pub fn leader_in_check(&self) -> bool {
        self.leader_in_check_for_pieces(&self.state.pieces)
    }
}
