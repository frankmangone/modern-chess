use serde::Serialize;
use std::collections::BTreeMap;

/// A node of the board: its name, and where a front end draws it, as the grid cell `[x, y]`
/// with `y = 0` at the bottom.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NodeView {
    pub name: String,
    pub at: [u8; 2],
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PieceView {
    pub position: String,
    pub code: String,
    pub player: String,
}

/// How the board's cells meet, so a front end knows whether to draw squares or hexes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    Square,
    Hex,
    Map,
}

/// How a hex board's cells are arranged: a wide diamond, or an upright hexagon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    Rhombus,
    Hexagon,
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
    /// Every node of the board, in board order. A cell that is not here is not on the board.
    pub nodes: Vec<NodeView>,
    /// Square or hex cells, over the grid `nodes` places them on.
    pub shape: Shape,
    /// On a hex board, how its cells are arranged; absent on a square board.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<Layout>,
    pub players: Vec<String>,
    pub pieces: Vec<PieceView>,
    pub turn: String,
    pub selected: Option<String>,
    pub destinations: Vec<String>,
    pub promotion: Option<Vec<String>>,
    pub status: Status,
    /// Zone name -> player -> names of the nodes, copied from the spec's POSITION conditions.
    pub zones: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    /// True when captured pieces go to a hand and can be dropped back (Shogi, Crazyhouse).
    pub hands: bool,
    /// Player -> piece code -> how many are waiting in their hand. Every player has an entry
    /// in a hand game, and the whole map is empty in any other game.
    pub hand: BTreeMap<String, BTreeMap<String, u32>>,
    /// For the player to move: piece code -> positions it may be dropped on. Only pieces they
    /// hold appear, and the map is empty whenever dropping is not possible (a promotion is
    /// pending, a jump must be finished, or the game is over).
    pub drops: BTreeMap<String, Vec<String>>,
    /// Player -> piece code -> how many of that player's pieces have been lost so far. Every
    /// player has an entry, empty until they lose something.
    pub captured: BTreeMap<String, BTreeMap<String, u32>>,
    /// Player -> what their pieces are worth, counting the board and the hand. Empty when the
    /// game gives its pieces no values.
    pub material: BTreeMap<String, u32>,
    /// Piece codes that can be placed any number of times without a hand (the stones in Go),
    /// sorted. Their squares are listed in `drops` like any dropped piece's.
    pub supply: Vec<String>,
    /// Whether the player to move may pass right now.
    pub can_pass: bool,
    /// How many turns in a row, up to now, were passes.
    pub passes: u32,
    /// Player -> score, counted as if the game ended now, when the game scores by area. Empty
    /// for any other game.
    pub score: BTreeMap<String, f64>,
}
