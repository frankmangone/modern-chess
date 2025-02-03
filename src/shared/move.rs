use crate::shared::Position;

/// A `BoardChange` is a poposed change on the board.
/// It's used for moves that have multiple effects on the board.
#[derive(Debug, Clone)]
pub struct BoardChange {
    pub position: Position,
    pub piece: String,
    // TODO: Also need to consider owning player.
}

#[derive(Debug, Clone)]
pub struct Move {
    pub target: Position,
    pub board_changes: Vec<BoardChange>
}
