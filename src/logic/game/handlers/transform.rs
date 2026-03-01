use crate::logic::{Game, GamePhase, GameError, Piece};

impl Game {
    /// Handle piece transformation.
    pub fn transform(&mut self, piece_code: String) -> Result<(), GameError> {
        match &self.state.phase {
            GamePhase::Transforming { position, options } => {
                if !options.contains(&piece_code) {
                    return Err(GameError::InvalidTransformationOption);
                }

                // Create new transformed piece
                let old_piece = self.state.pieces.get(position).unwrap();
                let new_piece = Piece::new(piece_code, old_piece.player.clone());

                // Apply the transformation
                self.state.pieces.remove(position);
                self.state.pieces.insert(position.clone(), new_piece);

                // Reset game state
                self.next_turn();
                self.clear_moves();
                self.check_game_over();
                Ok(())
            },
            _ => Err(GameError::InvalidGamePhase)
        }
    }
}
