use serde::{Deserialize, Serialize};
use super::condition::Condition;
use super::side_effect::SideEffect;

#[derive(Debug, Deserialize, Serialize)]
pub struct Action {
    state: String,

    action: String,

    #[serde(default)]
    conditions: Vec<Condition>,

    #[serde(default)]
    side_effects: Vec<SideEffect>,
}