use std::collections::HashMap;

use crate::logic::game::board_changes;
use crate::logic::{Game, Piece};
use crate::shared::{BoardChange, Effect, Position, ALLY_ON_FILE, DROP};
use crate::specs::ConditionSpec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DropRestrictionCode {
    AllyOnFile,
    Custom,
}

impl From<&str> for DropRestrictionCode {
    fn from(value: &str) -> Self {
        match value {
            ALLY_ON_FILE => Self::AllyOnFile,
            _ => Self::Custom,
        }
    }
}

impl Game {
    /// Returns true if the current player has at least one legal move (one that does not
    /// leave their leader in check). Short-circuits on the first legal move found.
    /// When hand_enabled, also checks whether any drop is available.
    pub fn any_legal_moves(&self) -> bool {
        let player = self.current_player_ref();

        // Check board moves.
        let has_board_move = self
            .state
            .pieces
            .iter()
            .filter(|(_, piece)| piece.player == player)
            .any(|(pos, piece)| {
                let Some(bp) = self.blueprints.get(&piece.code) else {
                    return false;
                };
                let Some(moves) = bp.calculate_moves(piece, pos, self) else {
                    return false;
                };
                moves.values().any(|effect| {
                    let sim =
                        board_changes::simulate_from(&self.state.pieces, &effect.board_changes);
                    !self.leader_in_check_for_pieces(&sim)
                })
            });

        if has_board_move {
            return true;
        }

        // Check drops (only relevant when hand_enabled).
        if !self.hand_enabled {
            return false;
        }

        self.state.hand.get(player).map_or(false, |hand| {
            hand.iter()
                .filter(|(_, count)| **count > 0)
                .any(|(piece_code, _)| !self.legal_drop_effects(piece_code, player).is_empty())
        })
    }

    /// Returns true if the drop restriction `cond` fires for the candidate drop square.
    /// A restriction firing means the drop is blocked at that square.
    pub fn check_drop_restriction(
        &self,
        position: &Position,
        cond: &ConditionSpec,
        current_player: &str,
    ) -> bool {
        match DropRestrictionCode::from(cond.condition.as_str()) {
            DropRestrictionCode::AllyOnFile => {
                let Some(ally_code) = &cond.piece else {
                    return false;
                };
                let file = position[0];
                self.state.pieces.iter().any(|(pos, p)| {
                    p.player == current_player && p.code == *ally_code && pos[0] == file
                })
            }
            // All other conditions are checked via the custom conditions map (e.g. POSITION).
            DropRestrictionCode::Custom => {
                self.check_position_condition_str(position, cond.condition.as_str())
            }
        }
    }

    /// Computes all legal drop squares for `piece_code` placed by `current_player`.
    pub fn compute_drop_squares(
        &self,
        piece_code: &str,
        current_player: &str,
    ) -> HashMap<Position, Effect> {
        self.legal_drop_effects(piece_code, current_player)
            .into_iter()
            .collect()
    }

    fn legal_drop_effects(
        &self,
        piece_code: &str,
        current_player: &str,
    ) -> Vec<(Position, Effect)> {
        let restrictions = self
            .blueprints
            .get(piece_code)
            .map(|bp| bp.drop_restrictions.clone())
            .unwrap_or_default();

        let mut available = Vec::new();
        for candidate in self.board.all_positions() {
            if self.state.pieces.contains_key(&candidate) {
                continue;
            }
            let blocked = restrictions
                .iter()
                .any(|cond| self.check_drop_restriction(&candidate, cond, current_player));
            if blocked {
                continue;
            }

            let new_piece = Piece::new(piece_code.to_string(), current_player.to_string());
            let board_changes = vec![BoardChange::set_piece(candidate.clone(), new_piece)];
            let sim = board_changes::simulate_from(&self.state.pieces, &board_changes);
            if self.leader_in_check_for_pieces(&sim) {
                continue;
            }
            available.push((
                candidate,
                Effect {
                    action: DROP.to_string(),
                    board_changes,
                    metadata: None,
                },
            ));
        }

        available
    }
}
