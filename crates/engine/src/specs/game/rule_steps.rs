//! Rules that look at every move: steps added to every action, and tests that narrow the ways.

use serde::{Deserialize, Serialize};

use super::action::DoSpec;

/// Steps run after an action's own steps, written `{ "for": [names], "do": [steps] }`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EveryActionSpec {
    /// The names of the actions it applies to. Absent means every action; an empty list is
    /// refused, as it would apply to none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#for: Option<Vec<String>>,
    /// Tests, effects and `if`s; a choice is refused.
    pub r#do: Vec<DoSpec>,
}

impl EveryActionSpec {
    /// Whether the entry applies to the action named `action`.
    pub fn applies_to(&self, action: &str) -> bool {
        self.r#for
            .as_ref()
            .is_none_or(|names| names.iter().any(|name| name == action))
    }
}
