use crate::logic::blueprint::move_blueprint::SideEffectBlueprint;
use crate::logic::{Piece, PieceState};
use crate::shared::BoardChange;

use super::context::SideEffectContext;

pub fn apply(
    side_effect: &SideEffectBlueprint,
    _ctx: &SideEffectContext<'_>,
    moved_piece: &mut Piece,
    _extra_changes: &mut Vec<BoardChange>,
) {
    if let Some(flag) = &side_effect.state {
        let value = match side_effect.duration {
            Some(duration) => PieceState::Uint(duration as u16),
            None => PieceState::Blank,
        };
        moved_piece.state.insert(flag.clone(), value);
    }
}
