use crate::logic::blueprint::move_blueprint::Condition;
use crate::shared::into_position;

use super::context::ConditionEvalContext;

pub fn eval(_condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    let Some(step) = ctx.blueprint.step.get(ctx.current_player()) else {
        return false;
    };

    let max_steps = step.iter().map(|&delta| delta.abs()).max().unwrap_or(0);
    if max_steps <= 1 {
        return true;
    }

    let unit: Vec<i16> = step.iter().map(|&delta| delta.signum()).collect();
    (1..max_steps).all(|distance| {
        let position: Vec<i16> = ctx
            .source_position
            .iter()
            .zip(unit.iter())
            .map(|(&src, &u)| src as i16 + u * distance)
            .collect();
        ctx.game.board.is_position_valid(&position)
            && ctx
                .game
                .piece_at_position(&into_position(&position))
                .is_none()
    })
}
