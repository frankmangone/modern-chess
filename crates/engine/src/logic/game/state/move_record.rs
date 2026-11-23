use serde::{Deserialize, Serialize};

use crate::logic::Answer;
use crate::shared::Position;

/// A record of a single turn's way, as it was played.
///
/// `piece_code`, `from` and `to` are read from the answers where they exist: the picked piece
/// (or the piece from the hand, or the piece put on the chosen place), its place, and the place
/// it went to. A place with no answer is empty.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveRecord {
    pub player: String,
    /// The action's name.
    pub action: String,
    /// The answers that chose the way (a promotion's code among them).
    pub chosen: Vec<Answer>,
    pub piece_code: String,
    pub from: Position,
    pub to: Position,
    /// Whether it removed a piece of another player.
    pub captures: bool,
    /// Whether it changed nothing.
    pub passed: bool,
}
