use serde::{Deserialize, Serialize};

/// Where a game instance is in its life. It only ever moves forward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceStatus {
    /// Waiting for players to take seats.
    Lobby,
    /// Every seat is taken and the game is being played.
    Active,
    /// Over. Terminal.
    Finished,
}

impl InstanceStatus {
    pub fn can_become(self, next: InstanceStatus) -> bool {
        matches!(
            (self, next),
            (InstanceStatus::Lobby, InstanceStatus::Active)
                | (InstanceStatus::Active, InstanceStatus::Finished)
        )
    }
}
