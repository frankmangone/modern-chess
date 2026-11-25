use crate::logic::{Game, GamePhase};

impl Game {
    /// Eliminates `player` from the turn order, ending the game when one player remains. It runs
    /// only from an end check, which is judged when a turn is handed on, so no `ASK` is waiting
    /// on anyone (`state.asked` is empty); a resignation ends the match in the server instead.
    pub(crate) fn eliminate_player(&mut self, player: &str) {
        let to_move = self.current_player();
        self.state.turn_order.retain(|p| p != player);
        // The cursor must stay inside the shrunken order even when the game ends here, or the
        // next look at whose turn it is reads past the end. When the player removed is the one
        // to move, the cursor now points at the next; when it is another, at the same player.
        let left = self.state.turn_order.len();
        if left > 0 {
            let cursor = if to_move == player {
                usize::from(self.state.current_turn)
            } else {
                self.state
                    .turn_order
                    .iter()
                    .position(|p| *p == to_move)
                    .unwrap_or(0)
            };
            self.state.current_turn = (cursor % left) as u8;
        }
        if self.state.turn_order.len() <= 1 {
            let winner = self.state.turn_order.first().cloned();
            self.state.phase = GamePhase::GameOver { winner };
            self.state.ways = Some(Vec::new());
        } else {
            let ways = self.compute_ways();
            self.start_choosing(ways);
        }
    }
}
