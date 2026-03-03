use crate::logic::blueprint::move_blueprint::Condition;
use crate::shared::into_position;

use super::{context::ConditionEvalContext, helpers};

pub fn eval(_condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    let Some(step) = ctx.blueprint.step.get(ctx.current_player()) else {
        return false;
    };

    let raw_target = helpers::absolute_position(ctx.source_position, step);
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
