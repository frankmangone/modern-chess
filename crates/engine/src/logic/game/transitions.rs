use crate::logic::{Game, GamePhase, Way};

impl Game {
    /// Starts the choices of the player to move with `ways`, nothing answered yet.
    pub(crate) fn start_choosing(&mut self, ways: Vec<Way>) {
        self.state.ways = Some(ways);
        self.state.phase = GamePhase::Choosing { chosen: Vec::new() };
    }
}
