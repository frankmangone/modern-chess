use crate::logic::blueprint::move_blueprint::Condition;
use crate::shared::into_position;

use super::{context::ConditionEvalContext, helpers};

pub fn eval(condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    // When `position` is set, check that specific square (offset from original_source).
    // When absent, fall back to the move's target square (source_position + step).
    let raw_target =
        if let Some(offset) = helpers::offset_for_player(condition, ctx.current_player()) {
            helpers::absolute_position(ctx.original_source, offset)
        } else {
            let Some(step) = ctx.blueprint.step.get(ctx.current_player()) else {
                return false;
            };
            helpers::absolute_position(ctx.source_position, step)
        };

    if !ctx.game.board.is_position_valid(&raw_target) {
        return false;
    }

    let target = into_position(&raw_target);
    !ctx.game
        .players
        .iter()
        .filter(|player| player.as_str() != ctx.current_player())
        .any(|opponent| ctx.game.attacked_by(opponent).contains(&target))
}
