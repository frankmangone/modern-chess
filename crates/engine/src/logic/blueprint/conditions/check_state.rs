use crate::logic::blueprint::move_blueprint::Condition;

use super::{context::ConditionEvalContext, helpers};

pub fn eval(condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    let Some(state_name) = &condition.state else {
        return false;
    };
    let Some(offset) = helpers::offset_for_player(condition, ctx.current_player()) else {
        return false;
    };

    let abs = helpers::absolute_position(ctx.source_position, offset);
    if !ctx.game.board.is_position_valid(&abs) {
        return false;
    }

    helpers::piece_at_absolute(ctx.game, &abs)
        .map_or(false, |piece| piece.state.contains_key(state_name.as_str()))
}
