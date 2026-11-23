use super::choose::stage_pin;
use crate::logic::{Game, GameError, GamePhase};

impl Game {
    /// Takes back the answers given so far this turn. Refused while a stage has its piece chosen, and
    /// once what the answers do is showing (a promotion after the move).
    pub fn cancel(&mut self) -> Result<(), GameError> {
        if !matches!(self.state.phase, GamePhase::Choosing { .. }) {
            return Err(GameError::InvalidGamePhase);
        }
        let pinned = self.state.ways.as_deref().and_then(stage_pin).is_some();
        if (self.state.stage.is_some() && pinned) || !self.shown_changes().is_empty() {
            return Err(GameError::CannotCancel);
        }
        self.state.phase = GamePhase::Choosing { chosen: Vec::new() };
        Ok(())
    }
}
