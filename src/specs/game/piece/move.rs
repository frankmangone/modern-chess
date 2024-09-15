use serde::{Deserialize, Serialize};
use super::action::ActionSpec;
use super::condition::ConditionSpec;
use super::side_effect::SideEffectSpec;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MoveSpec {
    id: u8,
    step: [i8; 2], // TODO: Maybe have this as a Vec for potentially more directions.
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