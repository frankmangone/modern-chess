use serde::{Deserialize, Serialize};

use crate::shared::Position;

/// A record of a single move made in the game.
///
/// `promotion` is `None` until the player resolves a `TRANSFORM` action;
/// `Game::transform()` fills it in on the same record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveRecord {
    pub player: String,
    pub piece_code: String,
    pub from: Position,
    pub to: Position,
    pub action: String,
    pub promotion: Option<String>,
}
