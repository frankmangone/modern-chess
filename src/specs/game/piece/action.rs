use serde::{Deserialize, Serialize};
use super::condition::ConditionSpec;
use super::side_effect::SideEffectSpec;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActionSpec {
    pub state: String,

    pub action: String,

    #[serde(default)]
    pub conditions: Vec<ConditionSpec>,

    #[serde(default)]
    pub side_effects: Vec<SideEffectSpec>,
}