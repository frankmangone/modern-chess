use crate::logic::{Game, GamePhase, Piece};

impl Game {
    /// Handle piece transformation.
    pub fn transform(&mut self, piece_code: String) -> Result<(), String> {
        match &self.state.phase {
            GamePhase::Transforming { position, options } => {
                if !options.contains(&piece_code) {
                    return Err("Invalid transformation option".to_string());
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
                self.state.phase = GamePhase::Idle;
                Ok(())
            },
            _ => Err("Game is not in transformation phase".to_string())
        }
    }
}
