use crate::logic::{Game, GamePhase, GameError};
use crate::shared::Position;

impl Game {
    /// Calculate moves for a specified position.
    /// Move calculation can only happen for the player that's currently playing.
    pub fn calculate_moves(&mut self, position: Position) -> Result<(), GameError> {
        let Some(piece) = self.state.pieces.get(&position) else {
            return Err(GameError::NoPieceInPosition);
        };

        if piece.player != self.current_player() {
            return Err(GameError::InvalidPlayer);
        }

        let Some(blueprint) = self.blueprints.get(&piece.code) else {
            return Err(GameError::NoAvailableMoves);
        };

        self.state.available_moves = blueprint.calculate_moves(piece, &position, self);
        self.state.phase = GamePhase::Moving { position };
        Ok(())
    }
}
