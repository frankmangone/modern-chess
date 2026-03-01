use crate::shared::Position;

#[derive(Debug, Clone, PartialEq)]
pub enum GamePhase {
    // No piece selected, waiting for player input
    Idle,

    // Piece selected, showing available moves
    Moving { position: Position },

    // Move selected, piece needs transformation
    Transforming {
        position: Position,
        options: Vec<String>
    },

    // Game is over; winner is Some(player) for checkmate, None for stalemate.
    GameOver {
        winner: Option<String>,
    },
}
