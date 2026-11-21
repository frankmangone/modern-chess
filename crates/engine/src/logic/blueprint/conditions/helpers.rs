use crate::logic::blueprint::move_blueprint::Condition;
use crate::logic::{Game, Offset};
use crate::shared::Position;

pub fn offset_for_player<'a>(condition: &'a Condition, current_player: &str) -> Option<&'a Offset> {
    let pos_map = condition.position.as_ref()?;
    pos_map.get(current_player)
}

/// The position one `offset` away from `from`, or `None` when that is off the board.
pub fn offset_position<'a>(
    game: &'a Game,
    from: &Position,
    offset: &Offset,
) -> Option<&'a Position> {
    game.board.step(from, offset.label)
}

pub fn count_bounds(condition: &Condition) -> (u8, u8) {
    (condition.min.unwrap_or(0), condition.max.unwrap_or(u8::MAX))
}
