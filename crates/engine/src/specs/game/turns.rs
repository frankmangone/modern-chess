use serde::{Deserialize, Serialize};

use super::test_block::TestSpec;

/// Turn spec, including playing order and starting position.
#[derive(Debug, Deserialize, Serialize)]
pub struct TurnSpec {
    /// The order in which turns will be executed. This vector is cycled around, and player names may repeat.
    pub order: Vec<String>,

    /// Index of the order where the game starts. For instance, if order is ['WHITE', 'BLACK'], we could set `start_at` to 1 so that
    /// BLACK starts instead of WHITE.
    #[serde(default = "default_start_at")]
    pub start_at: u8,

    /// The stages a way's `STAGE` effect can put the turn in.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stages: Vec<StageSpec>,
}

/// A stage of a turn: after a way that runs `{ "effect": "STAGE", "name": NAME }`, the same player
/// moves again, and only with the ways that pass `only`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StageSpec {
    pub name: String,
    /// A test judged on each way the stage could play next, on the position it leaves, with
    /// `START` the piece's place and `LAST` the place the way before ended. A way that fails it
    /// is not offered. Every way is allowed when it is not written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub only: Option<TestSpec>,
}

fn default_start_at() -> u8 {
    0u8
}

#[cfg(test)]
impl TurnSpec {
    /// A way to initialize a `TurnSpec`. Used only for tests.
    pub fn from_order(order: Vec<&str>) -> Self {
        TurnSpec {
            order: order.into_iter().map(|x| x.to_string()).collect(),
            start_at: 0u8,
            stages: Vec::new(),
        }
    }
}
