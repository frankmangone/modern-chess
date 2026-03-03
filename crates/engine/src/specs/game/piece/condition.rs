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

    /// For ALLY_ON_FILE, PIECE_AT, PIECE_NOT_AT, ALLY_ADJACENT_COUNT:
    /// the piece code to check for (e.g. "CANNON").
    #[serde(default)]
    pub piece: Option<String>,

    /// For PATH_PIECE_COUNT and ALLY_ADJACENT_COUNT: minimum count (inclusive, default 0).
    #[serde(default)]
    pub min: Option<u8>,

    /// For PATH_PIECE_COUNT and ALLY_ADJACENT_COUNT: maximum count (inclusive, default u8::MAX).
    #[serde(default)]
    pub max: Option<u8>,
}
