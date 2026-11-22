use std::collections::HashSet;

use crate::logic::blueprint::move_blueprint::Condition;
use crate::shared::Position;

use super::context::ConditionEvalContext;

/// Passes when no opponent attacks any square of the line from the current square to where the
/// step lands, that square included (castling's king path).
pub fn eval(_condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    let Some(step) = ctx.blueprint.step.get(ctx.current_player()) else {
        return false;
    };
    let board = &ctx.game.board;
    let Some(from) = board.node(ctx.source_position) else {
        return false;
    };
    let Some(to) = board.node(ctx.target_position) else {
        return false;
    };
    let Some(path) = board.path(from, to, step.unit) else {
        return false;
    };
    if path.is_empty() {
        return true;
    }

    let opponent_attacks: HashSet<Position> = ctx
        .game
        .rules
        .players
        .iter()
        .filter(|player| player.as_str() != ctx.current_player())
        .flat_map(|opponent| ctx.game.attacked_by(opponent))
        .collect();

    path.iter().all(|&node| {
        board
            .name(node)
            .is_some_and(|position| !opponent_attacks.contains(position))
    })
}
