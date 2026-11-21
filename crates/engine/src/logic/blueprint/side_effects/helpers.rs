use std::collections::HashMap;

use crate::logic::{Game, Offset};
use crate::shared::Position;

/// The position one of the player's offsets away from `source_position`, when it is on the
/// board.
pub fn relative_position(
    relative_by_player: Option<&HashMap<String, Offset>>,
    source_position: &Position,
    current_player: &str,
    game: &Game,
) -> Option<Position> {
    let offset = relative_by_player?.get(current_player)?;
    game.board.step(source_position, offset.label).cloned()
}
