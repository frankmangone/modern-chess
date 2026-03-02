use crate::shared::Position;

#[derive(Debug, Clone)]
pub enum GameTransition {
    // Calculate move for a position
    CalculateMoves { position: Position },

    // Execute a move
    ExecuteMove { position: Position },

    // Transform a piece 
    Transform { target: String }
}
