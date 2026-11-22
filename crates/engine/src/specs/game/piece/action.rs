use super::side_effect::SideEffectSpec;
use serde::{Deserialize, Serialize};

use crate::specs::game::old_forms;
use crate::specs::game::test_block::TestSpec;

/// A move's old occupancy-keyed action (`EMPTY → MOVE`), replaced by the move's `do`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LegacyActionSpec {
    pub state: String,

    pub action: String,

    /// Tests that must all hold, judged in order.
    #[serde(default, deserialize_with = "old_forms::tests")]
    pub conditions: Vec<TestSpec>,

    #[serde(default)]
    pub side_effects: Vec<SideEffectSpec>,
}
