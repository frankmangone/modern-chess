use crate::logic::blueprint::move_blueprint::SideEffectBlueprint;
use crate::logic::Piece;
use crate::shared::BoardChange;

use super::context::SideEffectContext;

pub fn apply(
    _side_effect: &SideEffectBlueprint,
    ctx: &SideEffectContext<'_>,
    _moved_piece: &mut Piece,
    extra_changes: &mut Vec<BoardChange>,
) {
    extra_changes.push(BoardChange::set_piece(
        ctx.original_source.clone(),
        ctx.acting_piece.clone(),
    ));
}
