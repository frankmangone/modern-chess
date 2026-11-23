use super::r#move::MoveSpec;
use serde::{Deserialize, Serialize};

use crate::specs::game::old_forms::{self, OldField};

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

    /// `drop_conditions`, replaced by tests in the drop action: refused when written.
    #[serde(
        rename = "drop_conditions",
        default,
        skip_serializing,
        deserialize_with = "old_forms::drop_conditions"
    )]
    pub old_drop_conditions: OldField,

    /// `drop_restrictions`, replaced by tests in the drop action: refused when written.
    #[serde(
        rename = "drop_restrictions",
        default,
        skip_serializing,
        deserialize_with = "old_forms::drop_restrictions"
    )]
    pub old_drop_restrictions: OldField,

    /// `restrictions`, replaced by an `IN_ZONE` test on each move: refused when written.
    #[serde(
        rename = "restrictions",
        default,
        skip_serializing,
        deserialize_with = "old_forms::restrictions"
    )]
    pub old_restrictions: OldField,

    /// `unlimited_supply`, replaced by an action that does `PLACE`: refused when written.
    #[serde(
        rename = "unlimited_supply",
        default,
        skip_serializing,
        deserialize_with = "old_forms::unlimited_supply"
    )]
    pub old_unlimited_supply: OldField,

    /// What the piece is worth when adding up the material on the board. A game in which no
    /// piece has a value does not report material at all.
    #[serde(default)]
    pub value: Option<u32>,
}
