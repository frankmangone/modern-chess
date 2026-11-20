use serde::{Deserialize, Serialize};

use super::SeatName;

/// Why a win happened, when it was not simply won on the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WinReason {
    /// The other side gave up.
    Resigned,
}

/// How a finished game ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Outcome {
    /// `reason` is empty for a win by play, so stored outcomes and their JSON are unchanged.
    Won {
        seat: SeatName,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<WinReason>,
    },
    Draw,
}
