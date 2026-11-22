use serde::{Deserialize, Serialize};

use crate::specs::game::test_block::ConditionEntry;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ModifierSpec {
    pub action: String,

    /// Tests that must all hold for the modifier to apply, judged with `START` the piece's
    /// square and `TARGET` the landing square.
    #[serde(default)]
    pub conditions: Vec<ConditionEntry>,

    #[serde(default)]
    pub options: Vec<String>,
}
