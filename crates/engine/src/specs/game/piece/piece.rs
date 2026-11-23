use super::r#move::MoveSpec;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PieceSpec {
    pub code: String,
    pub name: String,
    pub moves: Vec<MoveSpec>,

    /// The piece code that enters a hand when this piece is sent there (an effect's
    /// `removed_to`). `None` means the piece itself enters it. Promoted pieces point to their
    /// base form, e.g. `"PAWN"` on TOKIN.
    #[serde(default)]
    pub captured_as: Option<String>,

    /// What the piece is worth when adding up the material on the board. A game in which no
    /// piece has a value does not report material at all.
    #[serde(default)]
    pub value: Option<u32>,
}
