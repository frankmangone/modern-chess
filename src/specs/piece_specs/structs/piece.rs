use serde::{Deserialize, Serialize};
use super::r#move::MoveSpec;

#[derive(Debug, Deserialize, Serialize)]
pub struct PieceSpec {
    code: String,
    name: String,
    moves: Vec<MoveSpec>,
}
