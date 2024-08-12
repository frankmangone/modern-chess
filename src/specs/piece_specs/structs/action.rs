use serde::{Deserialize, Serialize};
use super::condition::ConditionSpec;
use super::side_effect::SideEffectSpec;

#[derive(Debug, Deserialize, Serialize)]
pub struct ActionSpec {
    state: String,

    action: String,

    #[serde(default)]
    conditions: Vec<ConditionSpec>,

    #[serde(default)]
    side_effects: Vec<SideEffectSpec>,
}