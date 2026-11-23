use super::choose::stage_pin;
use crate::logic::{Game, GameError, GamePhase};
use crate::shared::Position;

impl Game {
    /// The piece a stage has chosen already, when every way the stage offers starts with it (a
    /// jump in progress): the answers cannot be taken back, and the piece stays chosen.
    pub fn pinned_piece(&self) -> Option<Position> {
        self.state.stage.as_ref()?;
        self.state.ways.as_deref().and_then(stage_pin)
    }

    /// Takes back the answers given so far this turn. Refused while a stage has its piece chosen, and
    /// once what the answers do is showing (a promotion after the move).
    pub fn cancel(&mut self) -> Result<(), GameError> {
        if !matches!(self.state.phase, GamePhase::Choosing { .. }) {
            return Err(GameError::InvalidGamePhase);
        }
        if self.pinned_piece().is_some() || !self.shown_changes().is_empty() {
            return Err(GameError::CannotCancel);
        }
        self.state.phase = GamePhase::Choosing { chosen: Vec::new() };
        Ok(())
    }
}
