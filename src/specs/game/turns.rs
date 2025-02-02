use serde::{Deserialize, Serialize};

/// Turn spec, including playing order and starting position.
#[derive(Debug, Deserialize, Serialize)]
pub struct TurnSpec {
    /// The order in which turns will be executed. This vector is cycled around, and player names may repeat.
    pub order: Vec<String>,

    /// Index of the order where the game starts. For instance, if order is ['WHITE', 'BLACK'], we could set `start_at` to 1 so that
    /// BLACK starts instead of WHITE.
    #[serde(default = "default_start_at")]
    pub start_at: u8,
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
            start_at: 0u8
        }
    }
}
