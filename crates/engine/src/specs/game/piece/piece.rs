use super::condition::ConditionSpec;
use super::r#move::MoveSpec;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PieceSpec {
    pub code: String,
    pub name: String,
    pub moves: Vec<MoveSpec>,

    /// The piece code that enters the capturer's hand when this piece is captured.
    /// `None` means the piece itself (base code) enters the hand.
    /// Promoted pieces point to their base form, e.g. `"PAWN"` on TOKIN.
    #[serde(default)]
    pub demotes_to: Option<String>,

    /// Conditions that block a drop. If any fires for a candidate square, that square
    /// is excluded from legal drop targets. An empty list (the default) means the piece
    /// can be dropped on any empty square.
    #[serde(default)]
    pub drop_restrictions: Vec<ConditionSpec>,
}
