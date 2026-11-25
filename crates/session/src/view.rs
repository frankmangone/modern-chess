use modern_chess::logic::Choice;
use modern_chess::specs::PlaceKind;
use serde::ser::SerializeTuple;
use serde::{Serialize, Serializer};
use std::collections::BTreeMap;

/// A node of the board: its name, and where a front end draws it: the grid cell `[x, y]` with
/// `y = 0` at the bottom, or a map place's point. A map place also says what it is, and a path
/// its two ends or an area its outline; a grid node has none of those.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NodeView {
    pub name: String,
    #[serde(serialize_with = "whole_numbers_as_integers")]
    pub at: [f64; 2],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<PlaceKind>,
    /// A path's two points.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ends: Option<[String; 2]>,
    /// An area's points, in drawing order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub around: Option<Vec<String>>,
}

/// Writes a number with no fraction as an integer, so a grid's `[4, 0]` stays `[4,0]`.
fn whole_numbers_as_integers<S: Serializer>(
    at: &[f64; 2],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut tuple = serializer.serialize_tuple(2)?;
    for value in at {
        // Integers up to 2^53 convert exactly, so the cast loses nothing.
        if value.fract() == 0.0 && value.abs() < 9_007_199_254_740_992.0 {
            tuple.serialize_element(&(*value as i64))?;
        } else {
            tuple.serialize_element(value)?;
        }
    }
    tuple.end()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PieceView {
    pub position: String,
    pub code: String,
    /// The player it belongs to; empty when it belongs to nobody (a shared place's pieces).
    pub player: String,
    /// How many pieces of this kind a place that holds many keeps; absent for a single piece.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
}

/// A bag: a store of pieces off the board, like a hand. A player's own bag has an owner; a
/// shared one has none.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BagView {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// Piece code -> how many it keeps. Empty when the viewer may not see into it (`hidden`).
    pub pieces: BTreeMap<String, u32>,
    /// The viewer may not see what it keeps. Left out when they may.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,
    /// How many pieces a hidden bag keeps, when its rules say the count shows. Left out for a bag
    /// the viewer sees into, and for one that keeps its count too.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
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
    /// Square or hex cells over the grid `nodes` places them on, or a map's places.
    pub shape: Shape,
    /// On a map, the written links between points that no path place draws; absent otherwise.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<[String; 2]>,
    /// On a hex board, how its cells are arranged; absent on a square board.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<Layout>,
    pub players: Vec<String>,
    pub pieces: Vec<PieceView>,
    pub turn: String,
    /// What the player to move can answer next, and the answers given so far this turn. Empty
    /// for any viewer who is not the one to move.
    pub choice: Choice,
    pub status: Status,
    /// Zone name -> player -> names of the nodes, copied from the spec's zones (`zones`).
    pub zones: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    /// The game's bags, a hand being the bag `HAND` of each player. Empty when it has none.
    pub bags: Vec<BagView>,
    /// Player -> piece code -> how many of that player's pieces have been lost so far. Every
    /// player has an entry, empty until they lose something.
    pub captured: BTreeMap<String, BTreeMap<String, u32>>,
    /// Player -> what their pieces are worth, counting the board and the bags. Empty when the
    /// game gives its pieces no values.
    pub material: BTreeMap<String, u32>,
    /// How many turns in a row, up to now, were passes.
    pub passes: u32,
    /// Player -> score, counted as if the game ended now, when the game scores by area. Empty
    /// for any other game.
    pub score: BTreeMap<String, f64>,
}
