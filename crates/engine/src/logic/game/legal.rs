use crate::logic::Game;

impl Game {
    /// Whether the player to move has at least one way of taking an action, worked out from
    /// the board as it stands. A game with no ways has no legal move.
    pub fn any_legal_moves(&self) -> bool {
        !self.compute_ways().is_empty()
    }
}
