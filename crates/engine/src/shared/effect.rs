use crate::{logic::Piece, shared::Position};

/// A `BoardChange` is a poposed change on the board.
/// It's used for moves that have multiple effects on the board.
#[derive(Debug, Clone)]
pub struct BoardChange {
    pub position: Position,
    pub piece: Option<Piece>,
}

impl BoardChange {
    pub fn clear(position: &Position) -> Self {
        Self {
            position: position.clone(),
            piece: None,
        }
    }

    pub fn set_piece(position: Position, piece: Piece) -> Self {
        Self {
            position: position.clone(),
            piece: Some(piece),
        }
    }
}
