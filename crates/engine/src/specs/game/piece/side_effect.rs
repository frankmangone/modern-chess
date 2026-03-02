use serde::{Deserialize, Serialize};
use super::condition::ConditionSpec;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SideEffectSpec {
    pub action: String,

    #[serde(default)]
    pub state: Option<String>,

    #[serde(default)]
    pub duration: Option<u8>,

    #[serde(default)]
    pub condition: Option<ConditionSpec>,

    #[serde(default)]
    pub options: Option<Vec<String>>,

    #[serde(default)]
    pub piece: Option<String>,

    #[serde(default)]
    pub from: Option<[i8; 2]>,

    #[serde(default)]
    pub to: Option<[i8; 2]>,

    #[serde(default)]
    pub target: Option<[i8; 2]>,
}