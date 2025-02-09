use crate::{
    logic::{blueprint::move_blueprint::Modifier, Piece},
    shared::Position
};

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

/// An `Effect` is a proposed change on the board.
/// It's used to describe moves. Moves can affect multiple positions on the board.
#[derive(Debug, Clone)]
pub struct Effect {
    pub action: String,
    pub board_changes: Vec<BoardChange>,
    pub modifiers: Vec<Modifier>
}
