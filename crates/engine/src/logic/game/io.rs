use crate::logic::{Game, MoveRecord};

impl Game {
    /// Serializes the mutable runtime state to a JSON string.
    /// The spec-derived structure (blueprints, board, players, etc.) is not included;
    /// restore by calling `restore_state` on a `Game` built from the same spec.
    pub fn save_state(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.state)
    }

    /// Replaces the current game state with one previously produced by `save_state`.
    /// `available_moves` will be `None` after restore; call `CalculateMoves` to repopulate it.
    pub fn restore_state(&mut self, json: &str) -> Result<(), serde_json::Error> {
        self.state = serde_json::from_str(json)?;
        Ok(())
    }

    /// Returns the full move history as a read-only slice.
    pub fn history(&self) -> &[MoveRecord] {
        &self.state.history
    }
}
