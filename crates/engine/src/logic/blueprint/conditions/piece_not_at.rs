use crate::logic::blueprint::move_blueprint::Condition;

use super::{context::ConditionEvalContext, helpers};

pub fn eval(condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    let Some(forbidden_code) = &condition.piece_code else {
        return false;
    };
    let Some(offset) = helpers::offset_for_player(condition, ctx.current_player()) else {
        return false;
    };

    let Some(position) = helpers::offset_position(ctx.game, ctx.source_position, offset) else {
        return true; // off-board -> pass
    };

    ctx.game
        .piece_at_position_ref(position)
        .map_or(true, |piece| &piece.code != forbidden_code)
}
