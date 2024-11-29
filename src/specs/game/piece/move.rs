use serde::{Deserialize, Serialize};

use crate::shared::ExtendedPosition;

use super::action::ActionSpec;
use super::condition::ConditionSpec;
use super::side_effect::SideEffectSpec;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MoveSpec {
    pub id: u8,
    pub step: ExtendedPosition,
    actions: Vec<ActionSpec>,

    #[serde(default)]
    conditions: Vec<ConditionSpec>,

    #[serde(default)]
    side_effects: Vec<SideEffectSpec>,

    #[serde(default)]
    repeat: Option<RepeatSpec>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RepeatSpec {
    #[serde(default)]
    until: Option<String>,

    #[serde(default)]
    times: Option<u8>,
}
