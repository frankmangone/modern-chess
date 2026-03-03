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
    let Some(convert_position) = helpers::relative_position(
        side_effect.target.as_ref(),
        ctx.source_position,
        ctx.current_player,
        ctx.game,
    ) else {
        return;
    };

    let Some(existing) = ctx.game.piece_at_position(&convert_position) else {
        return;
    };
    if existing.player == ctx.current_player {
        return;
    }

    let code = side_effect
        .piece
        .as_deref()
        .unwrap_or(&ctx.acting_piece.code)
        .to_string();
    extra_changes.push(BoardChange::set_piece(
        convert_position,
        Piece::new(code, ctx.current_player.to_string()),
    ));
}
