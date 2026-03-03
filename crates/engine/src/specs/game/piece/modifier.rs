use super::condition::ConditionSpec;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ModifierSpec {
    pub action: String,

    #[serde(default)]
    pub conditions: Vec<ConditionSpec>,

    #[serde(default)]
    pub options: Vec<String>,
}
