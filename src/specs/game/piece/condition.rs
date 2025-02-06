use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConditionSpec {    
    pub condition: String,

    #[serde(default)]
    pub move_id: Option<u8>,

    #[serde(default)]
    pub state: Option<String>,

    #[serde(default)]
    pub position: Option<[i8; 2]>,
}