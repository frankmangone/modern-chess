use serde::{Deserialize, Serialize};
use super::condition::ConditionSpec;
use super::side_effect::SideEffectSpec;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActionSpec {
    pub state: String,

    pub action: String,

    #[serde(default)]
    conditions: Vec<ConditionSpec>,

    #[serde(default)]
    side_effects: Vec<SideEffectSpec>,
}