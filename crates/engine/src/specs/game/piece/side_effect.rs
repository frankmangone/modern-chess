use super::condition::ConditionSpec;
use serde::{Deserialize, Serialize};

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

    /// For EXPLODE: blast radius in squares (default 1).
    #[serde(default)]
    pub radius: Option<u8>,

    /// For EXPLODE: piece codes that are NOT removed by the blast on adjacent squares.
    /// The center square (landing square) is always cleared.
    #[serde(default)]
    pub exempt_codes: Vec<String>,
}
