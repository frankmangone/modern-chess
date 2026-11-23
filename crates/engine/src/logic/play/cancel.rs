use crate::logic::{Game, GameError, GamePhase};

impl Game {
    /// Takes back the answers given so far this turn. Refused while a jump is under way, and
    /// once what the answers do is showing (a promotion after the move).
    pub fn cancel(&mut self) -> Result<(), GameError> {
        if !matches!(self.state.phase, GamePhase::Choosing { .. }) {
            return Err(GameError::InvalidGamePhase);
        }
        if self.state.continuing.is_some() || !self.shown_changes().is_empty() {
            return Err(GameError::CannotCancel);
        }
        self.state.phase = GamePhase::Choosing { chosen: Vec::new() };
        Ok(())
    }
}
