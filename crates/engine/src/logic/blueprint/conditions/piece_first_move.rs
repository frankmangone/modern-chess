use crate::logic::blueprint::move_blueprint::Condition;

use super::{context::ConditionEvalContext, helpers};

pub fn eval(condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    let Some(offset) = helpers::offset_for_player(condition, ctx.current_player()) else {
        return condition.absent_ok;
    };

    let Some(position) = helpers::offset_position(ctx.game, ctx.source_position, offset) else {
        return condition.absent_ok;
    };

    ctx.game
        .piece_at_position_ref(position)
        .map_or(condition.absent_ok, |piece| piece.total_moves == 0)
}
