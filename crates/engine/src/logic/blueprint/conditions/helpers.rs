use crate::logic::blueprint::move_blueprint::Condition;
use crate::logic::{Game, Piece};
use crate::shared::{into_position, ExtendedPosition, Position};

pub fn offset_for_player<'a>(
    condition: &'a Condition,
    current_player: &str,
) -> Option<&'a ExtendedPosition> {
    let pos_map = condition.position.as_ref()?;
    pos_map.get(current_player)
}

pub fn absolute_position(
    source_position: &Position,
    offset: &ExtendedPosition,
) -> ExtendedPosition {
    source_position
        .iter()
        .zip(offset.iter())
        .map(|(&src, &delta)| src as i16 + delta)
        .collect()
}

pub fn piece_at_absolute(game: &Game, absolute_position: &ExtendedPosition) -> Option<Piece> {
    game.piece_at_position(&into_position(absolute_position))
}

pub fn count_bounds(condition: &Condition) -> (u8, u8) {
    (condition.min.unwrap_or(0), condition.max.unwrap_or(u8::MAX))
}
