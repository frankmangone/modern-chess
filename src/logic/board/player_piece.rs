use std::rc::Rc;

use crate::logic::Piece;

/// A `PlayerPiece` is simply that: a representation of a piece, belonging to
/// a player. This allows us to query for piece movements that depend on the issuing
/// player. For example, we don't calculate moves for pieces whose turn is not the current
/// turn.
#[derive(Debug)]
pub struct PlayerPiece {
    pub player: String,
    pub piece: Rc<Piece>
}