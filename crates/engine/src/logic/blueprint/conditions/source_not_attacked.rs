use crate::logic::blueprint::move_blueprint::Condition;

use super::context::ConditionEvalContext;

pub fn eval(_condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    !ctx.game
        .players
        .iter()
        .filter(|player| player.as_str() != ctx.current_player())
        .any(|opponent| ctx.game.attacked_by(opponent).contains(ctx.original_source))
}
