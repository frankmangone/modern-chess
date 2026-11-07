use serde::{Deserialize, Serialize};

use super::SeatName;

/// How a finished game ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Outcome {
    Won { seat: SeatName },
    Draw,
}
