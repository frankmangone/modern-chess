use crate::logic::blueprint::move_blueprint::Condition;

use super::{context::ConditionEvalContext, helpers};

pub fn eval(condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    // When `position` is set, check that specific square (offset from original_source).
    // When absent, fall back to the move's target square (source_position + step).
    let target = if let Some(offset) = helpers::offset_for_player(condition, ctx.current_player()) {
        helpers::offset_position(ctx.game, ctx.original_source, offset)
    } else {
        Some(ctx.target_position)
    };
    let Some(target) = target else {
        return false;
    };

    !ctx.game
        .rules
        .players
        .iter()
        .filter(|player| player.as_str() != ctx.current_player())
        .any(|opponent| ctx.game.attacked_by(opponent).contains(target))
}
