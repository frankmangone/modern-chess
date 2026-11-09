use crate::logic::blueprint::move_blueprint::Condition;

use super::{context::ConditionEvalContext, helpers};

pub fn eval(condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    let mut count = 0u8;

    for dx in -1i16..=1 {
        for dy in -1i16..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let [source_x, source_y, ..] = ctx.original_source.as_slice() else {
                continue;
            };

            let position = vec![*source_x as i16 + dx, *source_y as i16 + dy];
            if !ctx.game.board.is_position_valid(&position) {
                continue;
            }

            if let Some(piece) = helpers::piece_at_absolute(ctx.game, &position) {
                if piece.player == ctx.current_player()
                    && condition
                        .piece_code
                        .as_ref()
                        .map_or(true, |code| &piece.code == code)
                {
                    count += 1;
                }
            }
        }
    }

    let (min, max) = helpers::count_bounds(condition);
    count >= min && count <= max
}
