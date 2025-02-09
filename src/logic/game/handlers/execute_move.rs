use crate::logic::{Game, GamePhase, GameError};
use crate::shared::{Effect, EffectMetadata, Position, MOVE, CAPTURE, TRANSFORM};


impl Game {
    /// Execute a move that's in the `available_moves` vector.
    pub fn execute_move(&mut self, position: Position) -> Result<(), GameError> {
        match &self.state.phase {
            GamePhase::Moving { position: _ } => (),
            _ => {
                return Err(GameError::InvalidGamePhase);
            }
        }

        if self.state.available_moves.is_none() {
            return Err(GameError::NoAvailableMoves);
        }

        let effect = self.state.available_moves.as_ref().unwrap().get(&position);

        if effect.is_none() {
            // Move state machine back to move selection phase.
            self.state.phase = GamePhase::Idle;

            return Err(GameError::InvalidMove);
        }

        let effect = effect.unwrap();

        // Execute the move if no transformation needed
        self.apply_effect(&effect.clone(), &position);
        Ok(())
    }

    // Apply the effect of a move to the board.
    fn apply_effect(&mut self, effect: &Effect, target: &Position) {
        effect.board_changes.iter().for_each(|change| {
            match &change.piece {
                Some(piece) => {
                    self.state.pieces.insert(
                        change.position.clone(),
                        piece.clone(),
                    );
                },
                None => {
                    self.state.pieces.remove(&change.position);
                },
            }
        });

        // Depending on the action, we may need to do different things now.
        match effect.action.as_str() {
            MOVE | CAPTURE => {
                self.next_turn();
                self.clear_moves();
                self.state.phase = GamePhase::Idle;
            }
            TRANSFORM => {
                // Note: this only works if I'm completely certain of the metadata type.
                // Otherwise this panics.
                let EffectMetadata::Options(options) = effect.metadata.as_ref().unwrap();

                // Transition to transformation phase.
                self.state.phase = GamePhase::Transforming {
                    position: target.clone(),
                    options: options.clone()
                }
            },
            _ => (),
        }
    }
}
