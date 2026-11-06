use crate::logic::blueprint::move_blueprint::Condition;
use crate::shared::into_position;

use super::{context::ConditionEvalContext, helpers};

/// Passes when the square at the `position` offset from `source_position` is empty.
/// Off-board squares are treated as unblocked (vacuously true).
pub fn eval(condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    let Some(offset) = helpers::offset_for_player(condition, ctx.current_player()) else {
        return false;
    };
    let abs = helpers::absolute_position(ctx.source_position, offset);
    if !ctx.game.board.is_position_valid(&abs) {
        return true; // off-board cannot block
    }
    ctx.game
        .piece_at_position_ref(&into_position(&abs))
        .is_none()
}
