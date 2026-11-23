use serde::{Deserialize, Serialize};

use crate::logic::Answer;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GamePhase {
    /// The player to move is answering the choices of their turn. `chosen` holds the answers
    /// given so far; the turn's ways that they lead to are what can still be played.
    Choosing {
        #[serde(default)]
        chosen: Vec<Answer>,
    },

    // Game is over; winner is Some(player) for checkmate, None for stalemate.
    GameOver {
        winner: Option<String>,
    },
}
