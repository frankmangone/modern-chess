use serde::{Deserialize, Serialize};

use super::board::BoardSpec;
use super::player::PlayerSpec;

#[derive(Debug, Deserialize, Serialize)]
pub struct GameSpec {
    pub name: String,
    pub pieces: Vec<String>,
    pub board: BoardSpec,
    pub players: Vec<PlayerSpec>
}
