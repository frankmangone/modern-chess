use std::collections::HashMap;

use crate::logic::Game;
use crate::shared::{into_position, ExtendedPosition, Position};

pub fn relative_position(
    relative_by_player: Option<&HashMap<String, ExtendedPosition>>,
    source_position: &Position,
    current_player: &str,
    game: &Game,
) -> Option<Position> {
    let relative = relative_by_player?.get(current_player)?;
    let absolute: Vec<i16> = source_position
        .iter()
        .zip(relative.iter())
        .map(|(&src, &delta)| src as i16 + delta)
        .collect();
    if !game.board.is_position_valid(&absolute) {
        return None;
    }
    Some(into_position(&absolute))
}
