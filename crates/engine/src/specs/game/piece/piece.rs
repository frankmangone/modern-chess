use super::r#move::MoveSpec;
use serde::{Deserialize, Serialize};

use crate::specs::game::old_forms::{self, OldField};
use crate::specs::game::test_block::TestSpec;

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

    /// Tests that must all hold for a drop on a square, judged with `TARGET` the square;
    /// `START` is refused, as a drop has no moving piece. Empty (the default) allows every
    /// empty square.
    #[serde(default, deserialize_with = "old_forms::tests")]
    pub drop_conditions: Vec<TestSpec>,

    /// `drop_restrictions`, replaced by `drop_conditions`: refused when written.
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

    /// When `true`, any player may place this piece on an empty square at any time, as many
    /// times as they like, without it ever being in their hand (stones in Go, discs in Reversi).
    /// Placing is a drop, so `drop_conditions` apply. Defaults to `false`.
    #[serde(default)]
    pub unlimited_supply: bool,

    /// What the piece is worth when adding up the material on the board. A game in which no
    /// piece has a value does not report material at all.
    #[serde(default)]
    pub value: Option<u32>,
}
