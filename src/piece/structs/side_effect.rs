use serde::{Deserialize, Serialize};
use super::condition::Condition;

#[derive(Debug, Deserialize, Serialize)]
pub struct SideEffect {
    action: String,

    #[serde(default)]
    state: Option<String>,

    #[serde(default)]
    duration: Option<u8>,

    #[serde(default)]
    condition: Option<Condition>,

    #[serde(default)]
    options: Option<Vec<String>>,

    #[serde(default)]
    piece: Option<String>,

    #[serde(default)]
    from: Option<[i8; 2]>,

    #[serde(default)]
    to: Option<[i8; 2]>,

    #[serde(default)]
    target: Option<[i8; 2]>,
}