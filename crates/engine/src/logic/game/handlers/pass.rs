use crate::logic::{Game, GameError, GamePhase, MoveRecord};
use crate::shared::PASS;

impl Game {
    /// Gives up the current player's turn. Only games that set `allow_pass` accept it, and only
    /// when nothing is pending: a piece that was picked up (but is not mid-jump) is put back, while
    /// a promotion choice, a jump in progress or a finished game cannot be passed over.
    pub fn pass(&mut self) -> Result<(), GameError> {
        if !self.allow_pass {
            return Err(GameError::PassNotAllowed);
        }
        match &self.state.phase {
            GamePhase::Idle
            | GamePhase::Dropping { .. }
            | GamePhase::Moving { locked: false, .. } => {}
            _ => return Err(GameError::InvalidGamePhase),
        }

        let player = self.current_player();
        self.state.history.push(MoveRecord {
            player,
            piece_code: String::new(),
            from: Vec::new(),
            to: Vec::new(),
            action: PASS.to_string(),
            promotion: None,
        });

        self.next_turn();
        self.clear_moves();
        self.check_game_over();
        Ok(())
    }

    /// How many turns in a row, counting back from the latest, were passes.
    pub fn consecutive_passes(&self) -> usize {
        self.state
            .history
            .iter()
            .rev()
            .take_while(|record| record.action == PASS)
            .count()
    }
}
