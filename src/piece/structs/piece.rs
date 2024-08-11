use serde::{Deserialize, Serialize};
use super::r#move::Move;

#[derive(Debug, Deserialize, Serialize)]
pub struct Piece {
    code: String,
    name: String,
    moves: Vec<Move>,
}
