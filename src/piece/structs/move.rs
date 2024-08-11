use serde::{Deserialize, Serialize};
use super::action::Action;
use super::condition::Condition;
use super::side_effect::SideEffect;

#[derive(Debug, Deserialize, Serialize)]
pub struct Move {
    id: u8,
    step: [i8; 2], // TODO: Maybe have this as a Vec for potentially more directions.
    actions: Vec<Action>,

    #[serde(default)]
    conditions: Vec<Condition>,

    #[serde(default)]
    side_effects: Vec<SideEffect>,

    #[serde(default)]
    repeat: Option<Repeat>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Repeat {
    until: String,
}