use chrono::{DateTime, Utc};
use modern_chess_kernel::{Handle, UserId};
use serde::{Deserialize, Serialize};

use super::SeatName;

/// Someone sitting at a seat.
///
/// The handle is a copy taken from the player's token when they joined: this module cannot
/// look it up, and it should not change under a game in progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    pub seat: SeatName,
    pub user: UserId,
    pub handle: Handle,
    pub joined_at: DateTime<Utc>,
}
