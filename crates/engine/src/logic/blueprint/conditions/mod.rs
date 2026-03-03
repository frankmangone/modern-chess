mod ally_adjacent_count;
mod check_state;
pub mod context;
mod depends_on;
mod first_move;
mod helpers;
mod not_attacked;
mod opponent_not_in_check;
mod path_empty;
mod path_not_attacked;
mod path_piece_count;
mod piece_at;
mod piece_first_move;
mod piece_not_at;
mod rook_first_move;
mod source_not_attacked;

use crate::shared::{
    ALLY_ADJACENT_COUNT, CHECK_STATE, DEPENDS_ON, FIRST_MOVE, NOT_ATTACKED, OPPONENT_NOT_IN_CHECK,
    PATH_EMPTY, PATH_NOT_ATTACKED, PATH_PIECE_COUNT, PIECE_AT, PIECE_FIRST_MOVE, PIECE_NOT_AT,
    ROOK_FIRST_MOVE, SOURCE_NOT_ATTACKED,
};

use super::move_blueprint::Condition;
use context::ConditionEvalContext;

pub fn evaluate_condition(condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    match condition.code.as_str() {
        FIRST_MOVE => first_move::eval(condition, ctx),
        DEPENDS_ON => depends_on::eval(condition, ctx),
        CHECK_STATE => check_state::eval(condition, ctx),
        PIECE_FIRST_MOVE => piece_first_move::eval(condition, ctx),
        ROOK_FIRST_MOVE => rook_first_move::eval(condition, ctx),
        PATH_EMPTY => path_empty::eval(condition, ctx),
        NOT_ATTACKED => not_attacked::eval(condition, ctx),
        PATH_NOT_ATTACKED => path_not_attacked::eval(condition, ctx),
        SOURCE_NOT_ATTACKED => source_not_attacked::eval(condition, ctx),
        PATH_PIECE_COUNT => path_piece_count::eval(condition, ctx),
        PIECE_AT => piece_at::eval(condition, ctx),
        PIECE_NOT_AT => piece_not_at::eval(condition, ctx),
        ALLY_ADJACENT_COUNT => ally_adjacent_count::eval(condition, ctx),
        OPPONENT_NOT_IN_CHECK => opponent_not_in_check::eval(condition, ctx),
        _ => true, // unknown conditions pass silently for forward compatibility
    }
}
