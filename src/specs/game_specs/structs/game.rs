use serde::{Deserialize, Serialize};

use super::board::BoardSpec;
use super::player::PlayerSpec;

#[derive(Debug, Deserialize, Serialize)]
pub struct GameSpec {
    name: String,
    pieces: Vec<String>,
    board: BoardSpec,
    players: Vec<PlayerSpec>
}
