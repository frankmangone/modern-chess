use crate::logic::{Game, GamePhase, GameError, MoveRecord};
use crate::shared::{Effect, EffectMetadata, Position, MOVE, CAPTURE, TRANSFORM};


impl Game {
    /// Execute a move that's in the `available_moves` vector.
    pub fn execute_move(&mut self, position: Position) -> Result<(), GameError> {
        if !matches!(self.state.phase, GamePhase::Moving { .. }) {
            return Err(GameError::InvalidGamePhase);
        }

        // Extract the source position (set by calculate_moves) before any further borrows.
        let from = match &self.state.phase {
            GamePhase::Moving { position: src } => src.clone(),
            _ => unreachable!(),
        };

        let Some(moves) = &self.state.available_moves else {
            return Err(GameError::NoAvailableMoves);
        };

        let Some(effect) = moves.get(&position) else {
            // Move state machine back to move selection phase.
            self.state.phase = GamePhase::Idle;
            return Err(GameError::InvalidMove);
        };

        // Clone to release the immutable borrow on `self` before the mutable `apply_effect` call.
        let effect = effect.clone();
        self.apply_effect(&effect, &from, &position);
        Ok(())
    }

    // Apply the effect of a move to the board.
    fn apply_effect(&mut self, effect: &Effect, from: &Position, to: &Position) {
        // Capture piece info for the history record before board changes are applied.
        let (player, piece_code) = self.state.pieces.get(from)
            .map(|p| (p.player.clone(), p.code.clone()))
            .unwrap_or_default();

        for change in &effect.board_changes {
            match &change.piece {
                Some(piece) => { self.state.pieces.insert(change.position.clone(), piece.clone()); },
                None => { self.state.pieces.remove(&change.position); },
            }
        }

        // Append a history record for every concrete action.
        self.state.history.push(MoveRecord {
            player,
            piece_code,
            from: from.clone(),
            to: to.clone(),
            action: effect.action.clone(),
            promotion: None,
        });

        // Depending on the action, we may need to do different things now.
        match effect.action.as_str() {
            MOVE | CAPTURE => {
                self.next_turn();
                self.clear_moves();
                self.check_game_over();
            }
            TRANSFORM => {
                // Note: this only works if I'm completely certain of the metadata type.
                // Otherwise this panics.
                let EffectMetadata::Options(options) = effect.metadata.as_ref().unwrap();

                // Transition to transformation phase.
                self.state.phase = GamePhase::Transforming {
                    position: to.clone(),
                    options: options.clone()
                }
            },
            _ => (),
        }
    }
}
