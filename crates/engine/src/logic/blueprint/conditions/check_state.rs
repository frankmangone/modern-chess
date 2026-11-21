use crate::logic::blueprint::move_blueprint::Condition;

use super::{context::ConditionEvalContext, helpers};

pub fn eval(condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    let Some(state_name) = &condition.state else {
        return false;
    };
    let Some(offset) = helpers::offset_for_player(condition, ctx.current_player()) else {
        return false;
    };
    let Some(position) = helpers::offset_position(ctx.game, ctx.source_position, offset) else {
        return false;
    };

    ctx.game
        .piece_at_position_ref(position)
        .map_or(false, |piece| piece.state.contains_key(state_name.as_str()))
}
