use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct PlayerSpec {
    name: String,
    direction: Vec<i8>,
    starting_positions: Vec<PiecePositionSpec>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PiecePositionSpec {
    piece: String,
    positions: Vec<Vec<u8>>
}
