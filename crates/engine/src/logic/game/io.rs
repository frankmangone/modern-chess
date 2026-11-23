use crate::logic::{Game, GameState, MoveRecord};

impl Game {
    /// Serializes the mutable runtime state to a JSON string.
    /// The spec-derived structure (blueprints, board, players, etc.) is not included;
    /// restore by calling `restore_state` on a `Game` built from the same spec.
    pub fn save_state(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.state)
    }

    /// Replaces the current game state with one previously produced by `save_state`. The
    /// turn's ways, which are not saved, are worked out again.
    pub fn restore_state(&mut self, json: &str) -> Result<(), serde_json::Error> {
        self.restore(serde_json::from_str(json)?);
        Ok(())
    }

    /// Replaces the current game state with a saved one. Every way of restoring a game goes
    /// through here, so a state saved in an older shape is brought up to date in one place: a
    /// state saved without a turn order gets the spec's, and on a grid a piece's key made of
    /// numbers is read as the square it names. On a map keys are kept as they are: `"01"` is a
    /// place, not the square `"1"`. The turn's ways are worked out again, so the answers given
    /// so far and a jump under way are kept.
    pub fn restore(&mut self, state: GameState) {
        self.state = state;
        self.state.adopt_saved_hand();
        if self.board.places().is_none() {
            self.state.normalise_grid_keys();
        }
        if self.state.turn_order.is_empty() {
            self.state.turn_order = self.rules.turn_order.clone();
        }
        self.work_out_ways();
    }

    /// Whether the stage a restored state is in is one of the rules' and leaves the turn a way
    /// to play: a state naming an unknown stage, or a stage whose `only` admits nothing, was not
    /// saved under these rules. True when the state is in no stage, or the game is over.
    pub fn stage_fits_the_rules(&self) -> bool {
        let Some(stage) = &self.state.stage else {
            return true;
        };
        let known = self
            .rules
            .stages
            .iter()
            .any(|(name, _)| *name == stage.name);
        let playable = match &self.state.phase {
            crate::logic::GamePhase::Choosing { .. } => self
                .state
                .ways
                .as_ref()
                .is_some_and(|ways| !ways.is_empty()),
            crate::logic::GamePhase::GameOver { .. } => true,
        };
        known && playable
    }

    /// Returns the full move history as a read-only slice.
    pub fn history(&self) -> &[MoveRecord] {
        &self.state.history
    }
}
