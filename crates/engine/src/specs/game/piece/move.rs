use serde::{Deserialize, Serialize};

use crate::shared::ExtendedPosition;

use super::action::ActionSpec;
use super::condition::ConditionSpec;
use super::modifier::ModifierSpec;
use super::side_effect::SideEffectSpec;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MoveSpec {
    pub id: u8,
    pub step: ExtendedPosition,
    pub actions: Vec<ActionSpec>,

    #[serde(default)]
    pub conditions: Vec<ConditionSpec>,

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
    #[serde(default)]
    pub until: Option<String>,

    #[serde(default)]
    pub times: Option<u8>,

    #[serde(default)]
    #[serde(rename = "loop")]
    pub loop_move: bool,
}
