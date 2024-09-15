use serde::{Deserialize, Serialize};
use super::r#move::MoveSpec;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PieceSpec {
    pub code: String,
    pub name: String,
    pub moves: Vec<MoveSpec>,
}
