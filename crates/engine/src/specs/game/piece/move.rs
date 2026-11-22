use serde::{Deserialize, Serialize};

use crate::shared::ExtendedPosition;
use crate::specs::game::action::DoSpec;
use crate::specs::game::old_forms::{self, OldField};
use crate::specs::game::test_block::TestSpec;

/// Where a move's step goes: a vector on a grid (`[0, 1]`), or the label of a map's links
/// (`"next"`), which leads to every place the label links to.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum StepSpec {
    Vector(ExtendedPosition),
    Label(String),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MoveSpec {
    pub id: u8,
    pub step: StepSpec,
    /// What the move does, as steps run for each candidate `TARGET` with `START` the piece's
    /// square: tests on the target first, then the effects.
    pub r#do: Vec<DoSpec>,

    /// Restricts the move to the listed players. Empty (the default) means every player.
    #[serde(default)]
    pub players: Vec<String>,

    /// `actions`, replaced by `do`: refused when written.
    #[serde(
        rename = "actions",
        default,
        skip_serializing,
        deserialize_with = "old_forms::move_actions"
    )]
    pub old_actions: OldField,

    /// `conditions`, replaced by test steps in `do`: refused when written.
    #[serde(
        rename = "conditions",
        default,
        skip_serializing,
        deserialize_with = "old_forms::move_conditions"
    )]
    pub old_conditions: OldField,

    /// `modifiers`, replaced by steps in `do`: refused when written.
    #[serde(
        rename = "modifiers",
        default,
        skip_serializing,
        deserialize_with = "old_forms::move_modifiers"
    )]
    pub old_modifiers: OldField,

    /// `side_effects`, replaced by effect steps in `do`: refused when written.
    #[serde(
        rename = "side_effects",
        default,
        skip_serializing,
        deserialize_with = "old_forms::move_side_effects"
    )]
    pub old_side_effects: OldField,

    #[serde(default)]
    pub repeat: Option<RepeatSpec>,

    /// When `true`, this blueprint contributes to the attack map but generates no legal moves.
    /// Use it to model threat patterns that differ from legal movement (e.g. the Xiangqi
    /// flying-general rule, where the General's threat extends along the full rank/file even
    /// though it can only step one square legally).
    #[serde(default)]
    pub threat_only: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RepeatSpec {
    /// When the slide stops: judged on the next square (`TARGET`) before going on from it, with
    /// `START` the piece's square. Defaults to the next square being occupied.
    #[serde(
        default,
        deserialize_with = "old_forms::until",
        skip_serializing_if = "Option::is_none"
    )]
    pub until: Option<TestSpec>,

    #[serde(default)]
    pub times: Option<u8>,

    #[serde(default)]
    #[serde(rename = "loop")]
    pub loop_move: bool,
}
