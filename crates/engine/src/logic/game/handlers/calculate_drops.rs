use crate::logic::{Game, GamePhase, GameError};

impl Game {
    /// Calculate all legal drop squares for `piece_code` from the current player's hand.
    /// Transitions to `GamePhase::Dropping` and populates `available_moves`.
    pub fn calculate_drops(&mut self, piece_code: String) -> Result<(), GameError> {
        let current_player = self.current_player();

        // Verify the piece is in the current player's hand with count > 0.
        let count = self.state.hand
            .get(&current_player)
            .and_then(|h| h.get(&piece_code))
            .copied()
            .unwrap_or(0);
        if count == 0 {
            return Err(GameError::PieceNotInHand);
        }

        let available = self.compute_drop_squares(&piece_code, &current_player);

        self.state.available_moves = Some(available);
        self.state.phase = GamePhase::Dropping { piece_code };
        Ok(())
    }
}
