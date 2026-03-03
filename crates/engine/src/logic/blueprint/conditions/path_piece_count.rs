use crate::logic::blueprint::move_blueprint::Condition;
use crate::shared::into_position;

use super::{context::ConditionEvalContext, helpers};

pub fn eval(condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    let Some(step) = ctx.blueprint.step.get(ctx.current_player()) else {
        return false;
    };

    let target = helpers::absolute_position(ctx.source_position, step);
    if !ctx.game.board.is_position_valid(&target) {
        return false;
    }

    let diff: Vec<i16> = target
        .iter()
        .zip(ctx.original_source.iter())
        .map(|(&target_coord, &source_coord)| target_coord - source_coord as i16)
        .collect();
    let max_distance = diff.iter().map(|&delta| delta.abs()).max().unwrap_or(0);
    if max_distance == 0 {
        return false;
    }

    let unit: Vec<i16> = diff.iter().map(|&delta| delta.signum()).collect();
    let mut count = 0u8;
    for distance in 1..max_distance {
        let position: Vec<i16> = ctx
            .original_source
            .iter()
            .zip(unit.iter())
            .map(|(&src, &u)| src as i16 + u * distance)
            .collect();

        if !ctx.game.board.is_position_valid(&position) {
            continue;
        }

        if let Some(piece) = ctx.game.piece_at_position(&into_position(&position)) {
            let code_matches = condition
                .piece_code
                .as_ref()
                .map_or(true, |code| &piece.code == code);
            if code_matches {
                count += 1;
            }
        }
    }

    let (min, max) = helpers::count_bounds(condition);
    count >= min && count <= max
}
