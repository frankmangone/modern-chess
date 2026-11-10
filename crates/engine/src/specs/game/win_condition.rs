use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A win condition evaluated after every move/drop.
/// When one fires, the moving player wins immediately.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WinConditionSpec {
    /// Discriminator: "PIECE_IN_ZONE", "OPPONENT_BARE", "CHECK_COUNT", "AREA_SCORE".
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

    /// For AREA_SCORE: the piece codes that count towards a player's area. Empty means every
    /// piece counts.
    #[serde(default)]
    pub pieces: Vec<String>,

    /// For AREA_SCORE: when `true`, an empty region that touches the counted pieces of exactly
    /// one player also counts for that player (Go's area scoring). When `false` (the default)
    /// only the pieces themselves are counted (Reversi).
    #[serde(default)]
    pub territory: bool,

    /// For AREA_SCORE: points added to a player's score, keyed by player name (komi).
    #[serde(default)]
    pub bonus: HashMap<String, f64>,

    /// For AREA_SCORE: how many passes in a row end the game and trigger the count. Defaults to
    /// one pass by each player.
    #[serde(default)]
    pub after_passes: Option<u8>,
}
