mod ally_adjacent_count;
mod check_state;
pub mod context;
mod depends_on;
mod helpers;
mod leg_empty;
mod not_attacked;
mod path_not_attacked;
mod path_piece_count;
mod piece_at;
mod piece_first_move;
mod piece_not_at;

use super::move_blueprint::{Condition, ConditionKind};
use context::ConditionEvalContext;

pub fn evaluate_condition(condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    match &condition.kind {
        ConditionKind::DependsOn => depends_on::eval(condition, ctx),
        ConditionKind::CheckState => check_state::eval(condition, ctx),
        ConditionKind::PieceFirstMove => piece_first_move::eval(condition, ctx),
        ConditionKind::NotAttacked => not_attacked::eval(condition, ctx),
        ConditionKind::PathNotAttacked => path_not_attacked::eval(condition, ctx),
        ConditionKind::PathPieceCount => path_piece_count::eval(condition, ctx),
        ConditionKind::PieceAt => piece_at::eval(condition, ctx),
        ConditionKind::PieceNotAt => piece_not_at::eval(condition, ctx),
        ConditionKind::AllyAdjacentCount => ally_adjacent_count::eval(condition, ctx),
        ConditionKind::LegEmpty => leg_empty::eval(condition, ctx),
        // Named global conditions (POSITION type) declared in the spec's `conditions` array.
        // Evaluate against the move's target square (source_position + step).
        ConditionKind::Named(name) => {
            let Some(step) = ctx.blueprint.step.get(ctx.current_player()) else {
                return false;
            };
            let Some(target) = ctx.game.board.step(ctx.source_position, step.label) else {
                return false;
            };
            ctx.game
                .check_position_condition_for_player_str(target, name, ctx.current_player())
        }
    }
}
