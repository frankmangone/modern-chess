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
        if !self.asks_fit_the_rules() {
            return false;
        }
        let Some(stage) = &self.state.stage else {
            return true;
        };
        let known = self.rules.stages.iter().any(|rule| rule.name == stage.name);
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

    /// Whether the seats a restored state is waiting on (`asked`, and the stage the asker moves
    /// again in) are the rules' own: players still in the turn order, in stages the rules have, with
    /// the first seat's stage the one the turn is in. A state saved under other rules fails this,
    /// and would be a game no one could play.
    fn asks_fit_the_rules(&self) -> bool {
        let state = &self.state;
        let known =
            |name: &crate::shared::Symbol| self.rules.stages.iter().any(|rule| rule.name == *name);
        if state.together.is_some() {
            return state.asked.is_empty()
                && state.ask_then.is_none()
                && !matches!(state.phase, crate::logic::GamePhase::GameOver { .. })
                && self.together_fits_the_rules();
        }
        if state.asked.is_empty() {
            return state.ask_then.is_none();
        }
        let over = matches!(state.phase, crate::logic::GamePhase::GameOver { .. });
        !over
            && state
                .asked
                .iter()
                .all(|seat| state.turn_order.contains(&seat.player) && known(&seat.stage))
            && state.ask_then.as_ref().map_or(true, known)
            && state.stage.as_ref().map(|stage| stage.name)
                == state.asked.first().map(|seat| seat.stage)
    }

    /// Whether the seats an `ASK` that asks all at once is waiting on are the rules' own: players
    /// still in the turn order, in a stage the rules have, each owing a way that exists (a seat's
    /// answers so far lead to one, and a kept answer is one of its ways), and some seat still owing.
    fn together_fits_the_rules(&self) -> bool {
        let Some(together) = &self.state.together else {
            return true;
        };
        let known =
            |name: &crate::shared::Symbol| self.rules.stages.iter().any(|rule| rule.name == *name);
        if !known(&together.stage) || together.then.as_ref().is_some_and(|then| !known(then)) {
            return false;
        }
        let mut probe = self.clone();
        let mut owing = false;
        for seat in &together.seats {
            if !self.state.turn_order.contains(&seat.player) {
                return false;
            }
            let ways = probe.seat_ways(&seat.player);
            match &seat.locked {
                Some(locked) => {
                    if !ways
                        .iter()
                        .any(|way| way.action == locked.action && way.answers == locked.answers)
                    {
                        return false;
                    }
                }
                None => {
                    owing = true;
                    if !ways.iter().any(|way| way.follows(&seat.chosen)) {
                        return false;
                    }
                }
            }
        }
        owing
    }

    /// Returns the full move history as a read-only slice.
    pub fn history(&self) -> &[MoveRecord] {
        &self.state.history
    }
}
