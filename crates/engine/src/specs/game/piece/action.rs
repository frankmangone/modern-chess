use super::side_effect::SideEffectSpec;
use serde::{Deserialize, Serialize};

use crate::specs::game::test_block::ConditionEntry;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActionSpec {
    pub state: String,

    pub action: String,

    /// Tests that must all hold, judged in order.
    #[serde(default)]
    pub conditions: Vec<ConditionEntry>,

    #[serde(default)]
    pub side_effects: Vec<SideEffectSpec>,
}
