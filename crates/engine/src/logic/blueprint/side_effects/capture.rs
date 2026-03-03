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
    let Some(capture_position) = helpers::relative_position(
        side_effect.target.as_ref(),
        ctx.source_position,
        ctx.current_player,
        ctx.game,
    ) else {
        return;
    };

    extra_changes.push(BoardChange::clear(&capture_position));
}
