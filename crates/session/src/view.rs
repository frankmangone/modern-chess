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
    /// True when captured pieces go to a hand and can be dropped back (Shogi, Crazyhouse).
    pub hands: bool,
    /// Player -> piece code -> how many are waiting in their hand. Every player has an entry
    /// in a hand game, and the whole map is empty in any other game.
    pub hand: BTreeMap<String, BTreeMap<String, u32>>,
    /// For the player to move: piece code -> squares it may be dropped on. Only pieces they
    /// hold appear, and the map is empty whenever dropping is not possible (a promotion is
    /// pending, a jump must be finished, or the game is over).
    pub drops: BTreeMap<String, Vec<Square>>,
}
