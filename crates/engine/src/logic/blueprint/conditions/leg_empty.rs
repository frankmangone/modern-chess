use crate::logic::blueprint::move_blueprint::Condition;

use super::{context::ConditionEvalContext, helpers};

/// Passes when the square at the `position` offset from `source_position` is empty.
/// Off-board squares are treated as unblocked (vacuously true).
pub fn eval(condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    let Some(offset) = helpers::offset_for_player(condition, ctx.current_player()) else {
        return false;
    };
    let Some(position) = helpers::offset_position(ctx.game, ctx.source_position, offset) else {
        return true; // off-board cannot block
    };
    ctx.game.piece_at_position_ref(position).is_none()
}
