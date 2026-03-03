use crate::logic::blueprint::move_blueprint::Condition;

use super::context::ConditionEvalContext;

pub fn eval(condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    condition
        .move_id
        .map_or(false, |id| ctx.valid_move_ids.contains(&id))
}
