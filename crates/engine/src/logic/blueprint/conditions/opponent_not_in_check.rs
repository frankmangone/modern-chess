use crate::logic::blueprint::move_blueprint::Condition;

use super::context::ConditionEvalContext;

pub fn eval(_condition: &Condition, _ctx: &ConditionEvalContext<'_>) -> bool {
    // OPPONENT_NOT_IN_CHECK is handled as a post-filter in calculate_single_move;
    // pass here so it doesn't block move-level checks prematurely.
    true
}
