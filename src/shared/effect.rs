use crate::shared::Position;

/// A `BoardChange` is a poposed change on the board.
/// It's used for moves that have multiple effects on the board.
#[derive(Debug, Clone)]
pub struct BoardChange {
    pub position: Position,
    pub piece: Option<String>, // Piece code.
    pub player: Option<String>, // Player name.
}

impl BoardChange {
    pub fn clear(position: &Position) -> Self {
        Self {
            position: position.clone(),
            piece: None,
            player: None
        }
    }

    pub fn set_piece(position: Position, piece: String, player: String) -> Self {
        Self {
            position: position.clone(),
            piece: Some(piece),
            player: Some(player)
        }
    }
}

/// An `Effect` is a proposed change on the board.
/// It's used to describe moves. Moves can affect multiple positions on the board.
#[derive(Debug, Clone)]
pub struct Effect {
    pub action: String,
    pub board_changes: Vec<BoardChange>
}
