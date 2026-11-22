use serde::{Deserialize, Serialize};

use crate::specs::game::old_forms;
use crate::specs::game::test_block::TestSpec;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ModifierSpec {
    pub action: String,

    /// Tests that must all hold for the modifier to apply, judged with `START` the piece's
    /// square and `TARGET` the landing square.
    #[serde(default, deserialize_with = "old_forms::tests")]
    pub conditions: Vec<TestSpec>,

    #[serde(default)]
    pub options: Vec<String>,
}
