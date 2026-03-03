use crate::logic::blueprint::move_blueprint::Condition;

use super::context::ConditionEvalContext;

pub fn eval(_condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    ctx.piece.total_moves == 0
}
