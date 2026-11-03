use crate::logic::blueprint::move_blueprint::Condition;

use super::{context::ConditionEvalContext, helpers};

pub fn eval(condition: &Condition, ctx: &ConditionEvalContext<'_>) -> bool {
    // At least one of piece_code or player must be specified.
    if condition.piece_code.is_none() && condition.player.is_none() {
        return false;
    }
    let Some(offset) = helpers::offset_for_player(condition, ctx.current_player()) else {
        return false;
    };

    let abs = helpers::absolute_position(ctx.source_position, offset);
    if !ctx.game.board.is_position_valid(&abs) {
        return false;
    }

    let Some(piece) = helpers::piece_at_absolute(ctx.game, &abs) else {
        return false;
    };

    // Check piece code if specified.
    if let Some(expected_code) = &condition.piece_code {
        if &piece.code != expected_code {
            return false;
        }
    }

    // Check player relationship if specified.
    if let Some(player_filter) = &condition.player {
        let is_self = piece.player == ctx.current_player();
        match player_filter.as_str() {
            "SELF" => {
                if !is_self {
                    return false;
                }
            }
            "ENEMY" => {
                if is_self {
                    return false;
                }
            }
            "ANY" => {}
            _ => return false,
        }
    }

    true
}
