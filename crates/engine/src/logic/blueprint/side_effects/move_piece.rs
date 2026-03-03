use crate::logic::blueprint::move_blueprint::SideEffectBlueprint;
use crate::logic::Piece;
use crate::shared::BoardChange;

use super::{context::SideEffectContext, helpers};

pub fn apply(
    side_effect: &SideEffectBlueprint,
    ctx: &SideEffectContext<'_>,
    _moved_piece: &mut Piece,
    extra_changes: &mut Vec<BoardChange>,
) {
    let Some(from_position) = helpers::relative_position(
        side_effect.from.as_ref(),
        ctx.source_position,
        ctx.current_player,
        ctx.game,
    ) else {
        return;
    };
    let Some(to_position) = helpers::relative_position(
        side_effect.to.as_ref(),
        ctx.source_position,
        ctx.current_player,
        ctx.game,
    ) else {
        return;
    };

    if let Some(piece) = ctx.game.piece_at_position(&from_position) {
        extra_changes.push(BoardChange::clear(&from_position));
        extra_changes.push(BoardChange::set_piece(to_position, piece));
    }
}
