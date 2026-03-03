use std::collections::HashSet;

use crate::logic::blueprint::move_blueprint::Condition;
use crate::shared::{into_position, Position};

use super::context::ConditionEvalContext;

pub fn eval(_condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    let Some(step) = ctx.blueprint.step.get(ctx.current_player()) else {
        return false;
    };

    let max_steps = step.iter().map(|&delta| delta.abs()).max().unwrap_or(0);
    if max_steps <= 0 {
        return true;
    }

    let unit: Vec<i16> = step.iter().map(|&delta| delta.signum()).collect();
    let opponent_attacks: HashSet<Position> = ctx
        .game
        .players
        .iter()
        .filter(|player| player.as_str() != ctx.current_player())
        .flat_map(|opponent| ctx.game.attacked_by(opponent))
        .collect();

    (1..=max_steps).all(|distance| {
        let position: Vec<i16> = ctx
            .source_position
            .iter()
            .zip(unit.iter())
            .map(|(&src, &u)| src as i16 + u * distance)
            .collect();
        ctx.game.board.is_position_valid(&position)
            && !opponent_attacks.contains(&into_position(&position))
    })
}
