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
    ALLY_ADJACENT_COUNT, CHECK_STATE, DEPENDS_ON, NOT_ATTACKED, PATH_NOT_ATTACKED,
    PATH_PIECE_COUNT, PIECE_AT, PIECE_FIRST_MOVE, PIECE_NOT_AT,
};

use super::move_blueprint::Condition;
use context::ConditionEvalContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConditionCode {
    DependsOn,
    CheckState,
    PieceFirstMove,
    NotAttacked,
    PathNotAttacked,
    PathPieceCount,
    PieceAt,
    PieceNotAt,
    AllyAdjacentCount,
    Unknown,
}

impl From<&str> for ConditionCode {
    fn from(value: &str) -> Self {
        match value {
            DEPENDS_ON => Self::DependsOn,
            CHECK_STATE => Self::CheckState,
            PIECE_FIRST_MOVE => Self::PieceFirstMove,
            NOT_ATTACKED => Self::NotAttacked,
            PATH_NOT_ATTACKED => Self::PathNotAttacked,
            PATH_PIECE_COUNT => Self::PathPieceCount,
            PIECE_AT => Self::PieceAt,
            PIECE_NOT_AT => Self::PieceNotAt,
            ALLY_ADJACENT_COUNT => Self::AllyAdjacentCount,
            _ => Self::Unknown,
        }
    }
}

pub fn evaluate_condition(condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    match ConditionCode::from(condition.code.as_str()) {
        ConditionCode::DependsOn => depends_on::eval(condition, ctx),
        ConditionCode::CheckState => check_state::eval(condition, ctx),
        ConditionCode::PieceFirstMove => piece_first_move::eval(condition, ctx),
        ConditionCode::NotAttacked => not_attacked::eval(condition, ctx),
        ConditionCode::PathNotAttacked => path_not_attacked::eval(condition, ctx),
        ConditionCode::PathPieceCount => path_piece_count::eval(condition, ctx),
        ConditionCode::PieceAt => piece_at::eval(condition, ctx),
        ConditionCode::PieceNotAt => piece_not_at::eval(condition, ctx),
        ConditionCode::AllyAdjacentCount => ally_adjacent_count::eval(condition, ctx),
        ConditionCode::Unknown => true, // unknown conditions pass silently for forward compatibility
    }
}
