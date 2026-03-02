use crate::shared::Position;

#[derive(Debug, Clone)]
pub enum GameTransition {
    // Calculate move for a position
    CalculateMoves { position: Position },

    // Execute a move
    ExecuteMove { position: Position },

    // Transform a piece
    Transform { target: String },

    // Calculate legal drop squares for a hand piece
    CalculateDrops { piece_code: String },

    // Execute a drop at the given position (phase must be Dropping)
    ExecuteDrop { position: Position },
}
