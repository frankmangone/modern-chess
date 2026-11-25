mod build;
pub mod error;
pub(crate) mod io;
mod rules;
pub mod state;
mod turn;

use std::sync::Arc;

use crate::logic::pieces::piece::Piece;
use crate::logic::places::board::Board;
use crate::shared::Position;
use state::GameState;

pub use rules::Rules;

#[derive(Debug, Clone)]
pub struct Game {
    /// The rules the game was built with.
    pub rules: Arc<Rules>,

    // The `board` stores info about dimensions and disabled positions.
    pub board: Board,

    // `state` contains the actual game state.
    pub state: GameState,
}

impl Game {
    pub(crate) fn piece_at_position_ref(&self, position: &Position) -> Option<&Piece> {
        self.state.pieces.get(position)
    }

    /// Finds the piece at a given position. If no piece is present, return None.
    pub fn piece_at_position(&self, position: &Position) -> Option<Piece> {
        self.piece_at_position_ref(position).cloned()
    }

    /// Gives the match the seed of its chance, and works the turn's ways out again, since they may
    /// roll. Only a game that rolls or shuffles keeps it: for any other the state stays as it was,
    /// so no stored match of an old game grows a key. A game that rolls and is never given a seed
    /// plays the sequence of seed 0.
    pub fn set_seed(&mut self, seed: u64) {
        if !self.rules.chance {
            return;
        }
        self.state.seed = Some(seed);
        self.state.draws = 0;
        self.work_out_ways();
    }

    /// The value of `player`'s counter `name`; 0 when it was never changed.
    pub fn counter(&self, player: &str, name: &str) -> i64 {
        self.state
            .counters
            .get(player)
            .and_then(|counters| counters.get(name))
            .copied()
            .unwrap_or(0)
    }
}
