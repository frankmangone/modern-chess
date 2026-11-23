//! Whose turn it is, and starting a turn.

use crate::logic::{Game, GamePhase, Way};

impl Game {
    pub fn next_turn(&mut self) {
        let new_turn = self.state.current_turn + 1;

        if new_turn >= self.state.turn_order.len() as u8 {
            self.state.current_turn = 0;
        } else {
            self.state.current_turn = new_turn;
        }

        // Tick duration-tracked state flags on every piece.
        for piece in self.state.pieces.values_mut() {
            piece.tick_state_flags();
        }
    }

    /// The player whose turn it will be once the current player has finished.
    pub(crate) fn player_after_current(&self) -> &str {
        let next = (self.state.current_turn as usize + 1) % self.state.turn_order.len().max(1);
        self.state
            .turn_order
            .get(next)
            .or_else(|| self.state.turn_order.first())
            .map_or("", String::as_str)
    }

    pub(crate) fn current_player_ref(&self) -> &str {
        self.state
            .turn_order
            .get(self.state.current_turn as usize)
            .or_else(|| self.state.turn_order.first())
            .map_or("", String::as_str)
    }

    pub fn current_player(&self) -> String {
        self.current_player_ref().to_string()
    }

    /// Starts the choices of the player to move with `ways`, nothing answered yet.
    pub(crate) fn start_choosing(&mut self, ways: Vec<Way>) {
        self.state.ways = Some(ways);
        self.state.phase = GamePhase::Choosing { chosen: Vec::new() };
    }
}
