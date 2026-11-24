use std::collections::BTreeMap;

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
    /// The dice it rolled, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rolled: Vec<RolledRecord>,
    /// The piles it shuffled, by the name of the bag. Not their new order: that is what a hidden
    /// pile keeps from the players, and the match's seed already replays it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shuffled: Vec<String>,
}

/// A die as it landed: the bag it was rolled into, and the pieces its face put there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RolledRecord {
    /// The bag's place: `TRAY`, or `HAND@WHITE` for a bag each player has.
    pub bag: String,
    /// Piece code → how many, as the face is written.
    pub face: BTreeMap<String, u32>,
}
