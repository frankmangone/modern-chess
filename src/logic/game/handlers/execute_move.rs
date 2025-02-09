use crate::logic::{Game, GamePhase};
use crate::shared::{Effect, Position};

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
        
        // Check if this move requires transformation
        // TODO: We may want to treat this more like a state machine...
        if let Some(modifier) = effect.modifiers.iter().find(|m| m.action == "TRANSFORM") {
            // Store transformation state and return
            if let GamePhase::Moving { position } = &self.state.phase {
                self.state.phase = GamePhase::Transforming {
                    position: position.clone(),
                    options: modifier.options.clone(),
                };
                return Ok(());
            }
        }

        // Execute the move if no transformation needed
        self.apply_move_effect(&effect.clone());
        self.next_turn();
        self.clear_moves();
        self.state.phase = GamePhase::Idle;
        Ok(())
    }

    // Helper method to apply move effects
    fn apply_move_effect(&mut self, effect: &Effect) {
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
    }
}
