use crate::logic::game::board_changes;
use crate::logic::{Game, GameError, GamePhase, MoveRecord};
use crate::shared::{Position, DROP};

impl Game {
    /// Execute a drop at `position`. Phase must be `Dropping`.
    pub fn execute_drop(&mut self, position: Position) -> Result<(), GameError> {
        let piece_code = match &self.state.phase {
            GamePhase::Dropping { piece_code } => piece_code.clone(),
            _ => return Err(GameError::InvalidGamePhase),
        };

        let Some(moves) = &self.state.available_moves else {
            return Err(GameError::NoAvailableMoves);
        };

        let Some(effect) = moves.get(&position) else {
            self.set_idle_state();
            return Err(GameError::InvalidMove);
        };

        let effect = effect.clone();
        let current_player = self.current_player();

        // Apply board changes (places the piece, and removes whatever it captures).
        let before = self.piece_counts();
        board_changes::apply_to_map(&mut self.state.pieces, &effect.board_changes);
        self.record_losses(&before);

        // A piece from the hand leaves it; one from an unlimited supply does not.
        if !self.rules.unlimited_supply.contains(&piece_code) {
            if let Some(player_hand) = self.state.hand.get_mut(&current_player) {
                if let Some(count) = player_hand.get_mut(&piece_code) {
                    if *count > 1 {
                        *count -= 1;
                    } else {
                        player_hand.remove(&piece_code);
                    }
                }
            }
        }

        // Append history record. An empty `from` means "from hand".
        self.state.history.push(MoveRecord {
            player: current_player,
            piece_code: piece_code.clone(),
            from: Position::default(),
            to: position,
            action: DROP.to_string(),
            promotion: None,
        });

        self.next_turn();
        self.clear_moves();
        self.check_game_over();
        Ok(())
    }
}
