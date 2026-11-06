use serde::Serialize;
use std::collections::BTreeMap;

/// A square on the board: `[x, y]`, with `y = 0` at the bottom.
pub type Square = [u8; 2];

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PieceView {
    pub x: u8,
    pub y: u8,
    pub code: String,
    pub player: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum Status {
    Playing,
    Won { winner: String },
    Draw,
}

/// Everything a front end needs to draw the game. This is the wrapper's own
/// shape, deliberately independent of the engine's internal state type.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct View {
    pub width: u8,
    pub height: u8,
    pub disabled: Vec<Square>,
    pub players: Vec<String>,
    pub pieces: Vec<PieceView>,
    pub turn: String,
    pub selected: Option<Square>,
    pub destinations: Vec<Square>,
    pub promotion: Option<Vec<String>>,
    pub status: Status,
    /// Zone name -> player -> squares, copied from the spec's POSITION conditions.
    pub zones: BTreeMap<String, BTreeMap<String, Vec<Square>>>,
    pub hands: bool,
}
