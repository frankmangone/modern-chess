use crate::logic::{Game, GameError, GamePhase};
use crate::shared::{Position, CAPTURE};

impl Game {
    /// Calculate moves for a specified position.
    /// Move calculation can only happen for the player that's currently playing.
    /// When in a locked `Moving` phase (jump continuation), only the locked piece may be
    /// selected and only capture moves are shown.
    pub fn calculate_moves(&mut self, position: Position) -> Result<(), GameError> {
        // In a locked Moving phase, only the locked piece may be selected.
        match &self.state.phase {
            GamePhase::Moving {
                position: locked_pos,
                locked: true,
                ..
            } if position != *locked_pos => {
                return Err(GameError::InvalidPlayer);
            }
            _ => {}
        }

        let Some(piece) = self.state.pieces.get(&position) else {
            return Err(GameError::NoPieceInPosition);
        };

        if piece.player != self.current_player_ref() {
            return Err(GameError::InvalidPlayer);
        }

        let Some(blueprint) = self.blueprints.get(&piece.code) else {
            return Err(GameError::NoAvailableMoves);
        };

        let mut moves = blueprint
            .calculate_moves(piece, &position, self)
            .unwrap_or_default();

        // Board-wide rules (liberty captures, repeated positions) decide what each move does.
        moves = moves
            .into_iter()
            .filter_map(|(target, effect)| self.settle(effect).map(|settled| (target, settled)))
            .collect();

        // A move that leaves the player's own leader attacked is never legal.
        self.retain_leader_safe(&mut moves);

        // In a locked phase (jump continuation), only capture moves are valid.
        if matches!(
            self.state.phase,
            GamePhase::Moving {
                capture_only: true,
                ..
            }
        ) {
            moves.retain(|_, e| e.action == CAPTURE);
        } else if self.mandatory_capture && self.any_capture_exists() {
            // mandatory_capture: if any current-player piece has a capture, suppress non-captures.
            moves.retain(|_, e| e.action == CAPTURE);
        }

        self.enter_moving_phase(position, moves);
        Ok(())
    }

    /// Returns `true` if any piece owned by the current player has at least one capture move.
    fn any_capture_exists(&self) -> bool {
        let player = self.current_player_ref().to_string();
        self.state
            .pieces
            .iter()
            .filter(|(_, p)| p.player == player)
            .any(|(pos, piece)| {
                let Some(bp) = self.blueprints.get(&piece.code) else {
                    return false;
                };
                bp.calculate_moves(piece, pos, self)
                    .map_or(false, |moves| moves.values().any(|e| e.action == CAPTURE))
            })
    }
}
