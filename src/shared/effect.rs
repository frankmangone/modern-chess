use crate::shared::Position;

/// A `BoardChange` is a poposed change on the board.
/// It's used for moves that have multiple effects on the board.
#[derive(Debug, Clone)]
pub struct BoardChange {
    pub position: Position,
    pub piece: Option<String>, // Piece code.
    pub player: Option<String>, // Player name.
}

/// An `Effect` is a proposed change on the board.
/// It's used to describe moves. Moves can affect multiple positions on the board.
#[derive(Debug, Clone)]
pub struct Effect {
    pub action: String,
    pub board_changes: Vec<BoardChange>
}
