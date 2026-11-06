use super::condition::ConditionSpec;
use super::r#move::MoveSpec;
use serde::{Deserialize, Serialize};

/// A single zone restriction. The piece may only land on squares that satisfy all listed zones.
/// Each `zone` references a global `POSITION`-type condition by its `code`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ZoneRestrictionSpec {
    pub zone: String,
}

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

    /// Zone restrictions that confine the piece's legal move destinations. Every entry must be
    /// satisfied; if any zone does not contain the landing square the move is filtered out.
    /// Empty list (the default) means no zone confinement.
    #[serde(default)]
    pub restrictions: Vec<ZoneRestrictionSpec>,
}
