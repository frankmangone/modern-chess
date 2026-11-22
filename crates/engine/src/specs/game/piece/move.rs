use serde::{Deserialize, Serialize};

use crate::shared::ExtendedPosition;
use crate::specs::game::action::DoSpec;
use crate::specs::game::old_forms;
use crate::specs::game::test_block::TestSpec;

use super::action::LegacyActionSpec;
use super::modifier::ModifierSpec;
use super::side_effect::SideEffectSpec;

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
    /// The old occupancy-keyed actions, replaced by `do`. Absent on a move written with `do`.
    #[serde(default)]
    pub actions: Vec<LegacyActionSpec>,

    /// What the move does, as steps run for each candidate `TARGET` with `START` the piece's
    /// square. `None` on a move written in the old forms.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#do: Option<Vec<DoSpec>>,

    /// Restricts the move to the listed players. Empty (the default) means every player.
    #[serde(default)]
    pub players: Vec<String>,

    /// Tests that must all hold, judged in order.
    #[serde(default, deserialize_with = "old_forms::tests")]
    pub conditions: Vec<TestSpec>,

    #[serde(default)]
    pub modifiers: Vec<ModifierSpec>,

    #[serde(default)]
    pub side_effects: Vec<SideEffectSpec>,

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
