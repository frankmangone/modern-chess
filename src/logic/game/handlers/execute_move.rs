use crate::logic::{Game, GamePhase};
use crate::shared::{Effect, EffectMetadata, Position, MOVE, CAPTURE, TRANSFORM};

impl Game {
    /// Execute a move that's in the `available_moves` vector.
    pub fn execute_move(&mut self, position: Position) -> Result<(), String> {
        match &self.state.phase {
            GamePhase::Moving { position: _ } => (),
            _ => {
                return Err("Invalid game phase".to_string());
            }
        }

        if self.state.available_moves.is_none() {
            return Err("No available moves".to_string());
        }

        let effect = self.state.available_moves.as_ref().unwrap().get(&position);

        if effect.is_none() {
            // Move state machine back to move selection phase.
            self.state.phase = GamePhase::Idle;

            return Err("Invalid move".to_string());
        }

        let effect = effect.unwrap();

        // Execute the move if no transformation needed
        self.apply_move_effect(&effect.clone(), &position);
        Ok(())
    }

    // Helper method to apply move effects
    fn apply_move_effect(&mut self, effect: &Effect, target: &Position) {
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
