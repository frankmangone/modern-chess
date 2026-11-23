use crate::shared::IdMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod move_record;
pub mod piece;
pub mod view;

use crate::logic::{GamePhase, Way};
use crate::shared::Position;
use move_record::MoveRecord;
use piece::Piece;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    // Pieces in the game are stored in a hashmap for quick lookup.
    #[serde(with = "position_map")]
    pub pieces: IdMap<Position, Piece>,

    // The players still playing, in the order they play. Eliminated players leave it.
    #[serde(default)]
    pub turn_order: Vec<String>,

    // Current turn is stored as a cursor to the `turn_order` vector.
    pub current_turn: u8,

    /// The ways of the turn being played, worked out when it started (and on restore); the
    /// answers only filter them. Not saved.
    #[serde(skip)]
    pub ways: Option<Vec<Way>>,

    // Current phase of the game
    pub phase: GamePhase,

    /// The piece that must keep capturing (a jump under way), when there is one.
    #[serde(default)]
    pub continuing: Option<Position>,

    // Full move history, in order.
    pub history: Vec<MoveRecord>,

    // Canonical position keys recorded after each half-move, used for repetition detection.
    // Each entry is a deterministic string encoding of (active player, all pieces + their state).
    #[serde(default)]
    pub position_hashes: Vec<String>,

    // Pieces in each player's hand: player → piece_code → count.
    // Only populated when hand_enabled is true in the spec.
    #[serde(default)]
    pub hand: HashMap<String, HashMap<String, u32>>,

    /// The game's counters: player → counter name → value. A counter that is not here is 0.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub counters: HashMap<String, HashMap<String, i64>>,

    // Pieces each player has lost so far: player → piece_code → count. A piece counts as lost
    // when a move leaves fewer of its code on the board than before, and a promotion or a
    // castling rook that only changes place or form does not.
    #[serde(default)]
    pub captured: HashMap<String, HashMap<String, u32>>,
}

/// Serde module for the pieces map, keyed by position names.
///
/// JSON object keys must be strings, so the pieces are written under their position's name
/// (`"4,2"`). Keys are read as they are written; `Game::restore` then reads a key made of
/// numbers as the grid square it names (`" 4, 2"` loads as `"4,2"`, as it always has), and only
/// on a grid, since a map's place may be named `"01"`.
mod position_map {
    use serde::de::{Deserializer, MapAccess, Visitor};
    use serde::ser::{SerializeMap, Serializer};
    use std::fmt;

    use crate::logic::Piece;
    use crate::shared::{IdMap, Position};

    pub fn serialize<S>(map: &IdMap<Position, Piece>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ser_map = s.serialize_map(Some(map.len()))?;
        for (pos, piece) in map {
            ser_map.serialize_entry(pos.as_str(), piece)?;
        }
        ser_map.end()
    }

    pub fn deserialize<'de, D>(d: D) -> Result<IdMap<Position, Piece>, D::Error>
    where
        D: Deserializer<'de>,
    {
        d.deserialize_map(PositionMapVisitor)
    }

    struct PositionMapVisitor;

    impl<'de> Visitor<'de> for PositionMapVisitor {
        type Value = IdMap<Position, Piece>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a map keyed by position names")
        }

        fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
            let mut map = IdMap::default();
            while let Some((key, value)) = access.next_entry::<String, Piece>()? {
                map.insert(Position::from(key), value);
            }
            Ok(map)
        }
    }
}

impl GameState {
    /// Reads every piece's key made of numbers as the grid square it names, so `" 4, 2"`
    /// becomes `"4,2"`, as states saved that way have always loaded. Any other key is a name
    /// and stays as it is.
    pub fn normalise_grid_keys(&mut self) {
        self.pieces = std::mem::take(&mut self.pieces)
            .into_iter()
            .map(|(position, piece)| (grid_position(position), piece))
            .collect();
    }
}

fn grid_position(position: Position) -> Position {
    let square: Option<Vec<u8>> = position
        .as_str()
        .split(',')
        .map(|part| part.trim().parse::<u8>().ok())
        .collect();
    match square {
        Some(square) => Position::from_coordinates(&square),
        None => position,
    }
}
