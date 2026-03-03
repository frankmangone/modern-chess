mod ally_adjacent_count;
mod check_state;
pub mod context;
mod depends_on;
mod helpers;
mod not_attacked;
mod path_not_attacked;
mod path_piece_count;
mod piece_at;
mod piece_first_move;
mod piece_not_at;

use crate::shared::{
    ALLY_ADJACENT_COUNT, CHECK_STATE, DEPENDS_ON, NOT_ATTACKED, PATH_NOT_ATTACKED, PATH_PIECE_COUNT,
    PIECE_AT, PIECE_FIRST_MOVE, PIECE_NOT_AT,
};

use super::move_blueprint::Condition;
use context::ConditionEvalContext;

pub fn evaluate_condition(condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    match condition.code.as_str() {
        DEPENDS_ON => depends_on::eval(condition, ctx),
        CHECK_STATE => check_state::eval(condition, ctx),
        PIECE_FIRST_MOVE => piece_first_move::eval(condition, ctx),
        NOT_ATTACKED => not_attacked::eval(condition, ctx),
        PATH_NOT_ATTACKED => path_not_attacked::eval(condition, ctx),
        PATH_PIECE_COUNT => path_piece_count::eval(condition, ctx),
        PIECE_AT => piece_at::eval(condition, ctx),
        PIECE_NOT_AT => piece_not_at::eval(condition, ctx),
        ALLY_ADJACENT_COUNT => ally_adjacent_count::eval(condition, ctx),
        _ => true, // unknown conditions pass silently for forward compatibility
    }
}
