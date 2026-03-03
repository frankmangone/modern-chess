use crate::logic::{Game, Piece};
use crate::shared::Position;

pub struct SideEffectContext<'a> {
    pub game: &'a Game,
    pub current_player: &'a str,
    pub acting_piece: &'a Piece,
    pub original_source: &'a Position,
    pub source_position: &'a Position,
}

impl<'a> SideEffectContext<'a> {
    pub fn new(
        game: &'a Game,
        current_player: &'a str,
        acting_piece: &'a Piece,
        original_source: &'a Position,
        source_position: &'a Position,
    ) -> Self {
        Self {
            game,
            current_player,
            acting_piece,
            original_source,
            source_position,
        }
    }
}
