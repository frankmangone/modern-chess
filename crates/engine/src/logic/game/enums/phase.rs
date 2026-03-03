use crate::shared::Position;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GamePhase {
    // No piece selected, waiting for player input
    Idle,

    // Piece selected, showing available moves
    Moving {
        position: Position,
    },

    // Move selected, piece needs transformation
    Transforming {
        position: Position,
        options: Vec<String>,
    },

    // Drop-piece selected from hand, showing legal drop squares.
    Dropping {
        piece_code: String,
    },

    // Game is over; winner is Some(player) for checkmate, None for stalemate.
    GameOver {
        winner: Option<String>,
    },
}
