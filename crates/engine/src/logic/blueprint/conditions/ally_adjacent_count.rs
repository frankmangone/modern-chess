use crate::logic::blueprint::move_blueprint::Condition;

use super::{context::ConditionEvalContext, helpers};

/// Counts the current player's pieces (of `piece_code`, when given) on the positions next to
/// the piece's own, along every axis of the board.
pub fn eval(condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    let mut count = 0u8;

    for position in ctx.game.board.positions_within(ctx.original_source, 1) {
        if position == *ctx.original_source {
            continue;
        }
        if let Some(piece) = ctx.game.piece_at_position_ref(&position) {
            if piece.player == ctx.current_player()
                && condition
                    .piece_code
                    .as_ref()
                    .map_or(true, |code| &piece.code == code)
            {
                count = count.saturating_add(1);
            }
        }
    }

    let (min, max) = helpers::count_bounds(condition);
    count >= min && count <= max
}
