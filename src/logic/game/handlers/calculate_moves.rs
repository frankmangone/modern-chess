use crate::logic::{Game, GamePhase};
use crate::shared::Position;

impl Game {
    /// Calculate moves for a specified position.
    /// Move calculation can only happen for the player that's currently playing.
    pub fn calculate_moves(&mut self, position: Position) -> Result<(), String> {
        let maybe_piece = self.state.pieces.get(&position);

        match maybe_piece {
            Some(piece) => {
                if piece.player != self.current_player() {
                    return Err("Invalid player".to_string());
                }

                match self.blueprints.get(&piece.code) {
                    Some(blueprint) => {
                        self.state.available_moves = blueprint.calculate_moves(&piece, &position, &self);
                        self.state.phase = GamePhase::Moving { 
                             position 
                        };
                        Ok(())
                    }
                    None => Err("No moves available".to_string())
                }
            },
            None => Err("No piece in selected position".to_string())
        }
    }
}
