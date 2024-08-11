use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Condition {    
    condition: String,

    #[serde(default)]
    move_id: Option<u8>,

    #[serde(default)]
    state: Option<String>,

    #[serde(default)]
    position: Option<[i8; 2]>,
}