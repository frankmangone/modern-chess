use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod move_record;
pub mod piece;

use crate::logic::GamePhase;
use crate::shared::{Effect, Position};
use move_record::MoveRecord;
use piece::Piece;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    // Pieces in the game are stored in a hashmap for quick lookup.
    #[serde(with = "position_map")]
    pub pieces: HashMap<Position, Piece>,

    // The players still playing, in the order they play. Eliminated players leave it.
    #[serde(default)]
    pub turn_order: Vec<String>,

    // Current turn is stored as a cursor to the `turn_order` vector.
    pub current_turn: u8,

    // Available moves are derived state; skipped during serialization and
    // left as None on restore. Recompute by calling CalculateMoves.
    #[serde(skip)]
    pub available_moves: Option<HashMap<Position, Effect>>,

    // Current phase of the game
    pub phase: GamePhase,

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

    // Per-player check counters used by the CHECK_COUNT win condition.
    // player → number of checks that player has delivered to opponents.
    #[serde(default)]
    pub check_counts: HashMap<String, u32>,

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
    use std::collections::HashMap;
    use std::fmt;

    use crate::logic::Piece;
    use crate::shared::Position;

    pub fn serialize<S>(map: &HashMap<Position, Piece>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ser_map = s.serialize_map(Some(map.len()))?;
        for (pos, piece) in map {
            ser_map.serialize_entry(pos.as_str(), piece)?;
        }
        ser_map.end()
    }

    pub fn deserialize<'de, D>(d: D) -> Result<HashMap<Position, Piece>, D::Error>
    where
        D: Deserializer<'de>,
    {
        d.deserialize_map(PositionMapVisitor)
    }

    struct PositionMapVisitor;

    impl<'de> Visitor<'de> for PositionMapVisitor {
        type Value = HashMap<Position, Piece>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a map keyed by position names")
        }

        fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
            let mut map = HashMap::new();
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
