use crate::logic::{Game, GameError, GamePhase, MoveRecord};
use crate::shared::DROP;

impl Game {
    /// Execute a drop at `position`. Phase must be `Dropping`.
    pub fn execute_drop(&mut self, position: Vec<u8>) -> Result<(), GameError> {
        let piece_code = match &self.state.phase {
            GamePhase::Dropping { piece_code } => piece_code.clone(),
            _ => return Err(GameError::InvalidGamePhase),
        };

        let Some(moves) = &self.state.available_moves else {
            return Err(GameError::NoAvailableMoves);
        };

        let Some(effect) = moves.get(&position) else {
            self.state.phase = GamePhase::Idle;
            return Err(GameError::InvalidMove);
        };

        let effect = effect.clone();
        let current_player = self.current_player();

        // Apply board changes (places the piece on the board).
        for change in &effect.board_changes {
            match &change.piece {
                Some(p) => {
                    self.state.pieces.insert(change.position.clone(), p.clone());
                }
                None => {
                    self.state.pieces.remove(&change.position);
                }
            }
        }

        // Decrement hand count; remove the entry if it reaches 0.
        if let Some(player_hand) = self.state.hand.get_mut(&current_player) {
            if let Some(count) = player_hand.get_mut(&piece_code) {
                if *count > 1 {
                    *count -= 1;
                } else {
                    player_hand.remove(&piece_code);
                }
            }
        }

        // Append history record. `from` is an empty Vec to indicate "from hand".
        self.state.history.push(MoveRecord {
            player: current_player,
            piece_code: piece_code.clone(),
            from: vec![],
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
