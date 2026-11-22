use crate::logic::blueprint::move_blueprint::Condition;

use super::{context::ConditionEvalContext, helpers};

/// Counts the pieces (of `piece_code`, when given) standing on the line between the piece's
/// own square and the end of the path, both ends left out.
pub fn eval(condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    // The path ends at the condition's `position` (relative to the piece's starting square)
    // when given, and otherwise at the square the move's step lands on.
    let (end, unit) = match helpers::offset_for_player(condition, ctx.current_player()) {
        Some(offset) => (
            helpers::offset_position(ctx.game, ctx.original_source, offset),
            offset.unit,
        ),
        None => {
            let Some(step) = ctx.blueprint.step.get(ctx.current_player()) else {
                return false;
            };
            (Some(ctx.target_position), step.unit)
        }
    };
    let board = &ctx.game.board;
    let (Some(from), Some(to)) = (
        board.node(ctx.original_source),
        end.and_then(|end| board.node(end)),
    ) else {
        return false;
    };
    if from == to {
        return false;
    }
    let Some(path) = board.path(from, to, unit) else {
        return false;
    };

    let mut count = 0u8;
    for &node in path.iter().take(path.len().saturating_sub(1)) {
        let Some(piece) = board
            .name(node)
            .and_then(|position| ctx.game.piece_at_position_ref(position))
        else {
            continue;
        };
        let code_matches = condition
            .piece_code
            .as_ref()
            .map_or(true, |code| &piece.code == code);
        if code_matches {
            count = count.saturating_add(1);
        }
    }

    let (min, max) = helpers::count_bounds(condition);
    count >= min && count <= max
}
