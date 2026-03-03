use serde::{Deserialize, Serialize};

/// A win condition evaluated after every move/drop.
/// When one fires, the moving player wins immediately.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WinConditionSpec {
    /// Discriminator: "PIECE_IN_ZONE", "OPPONENT_BARE", "CHECK_COUNT".
    pub r#type: String,

    /// For PIECE_IN_ZONE: the piece code that must reach the zone (e.g. "KING").
    #[serde(default)]
    pub piece: Option<String>,

    /// For PIECE_IN_ZONE: the name of a global POSITION condition defining the win zone.
    #[serde(default)]
    pub zone: Option<String>,

    /// For OPPONENT_BARE: piece codes that are exempt from the "bare" check.
    /// Win is triggered when every opponent has only pieces whose codes are in this list.
    #[serde(default)]
    pub exempt: Vec<String>,

    /// For CHECK_COUNT: number of checks needed to win (default 3).
    #[serde(default)]
    pub threshold: Option<u32>,
}
