use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::shared::{Effect, Position};
use crate::logic::Piece;
use crate::logic::GamePhase;
use crate::logic::MoveRecord;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    // Pieces in the game are stored in a hashmap for quick lookup.
    #[serde(with = "position_map")]
    pub pieces: HashMap<Position, Piece>,

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
}

/// Serde module for `HashMap<Position, V>` where `Position = Vec<u8>`.
///
/// JSON object keys must be strings, so each `Vec<u8>` key is serialized as
/// a comma-separated string (e.g. `[4, 2]` → `"4,2"`) using the existing
/// `into_string` helper, and parsed back symmetrically on deserialize.
mod position_map {
    use std::collections::HashMap;
    use serde::de::{Deserializer, MapAccess, Visitor};
    use serde::ser::{SerializeMap, Serializer};
    use std::fmt;
    use std::str::FromStr;

    use crate::shared::{into_string, Position};
    use crate::logic::Piece;

    pub fn serialize<S>(map: &HashMap<Position, Piece>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ser_map = s.serialize_map(Some(map.len()))?;
        for (pos, piece) in map {
            ser_map.serialize_entry(&into_string(pos), piece)?;
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
            f.write_str("a map with comma-separated u8 string keys")
        }

        fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
            let mut map = HashMap::new();
            while let Some((key, value)) = access.next_entry::<String, Piece>()? {
                let pos: Position = key
                    .split(',')
                    .map(|s| u8::from_str(s.trim()).map_err(serde::de::Error::custom))
                    .collect::<Result<Vec<u8>, _>>()?;
                map.insert(pos, value);
            }
            Ok(map)
        }
    }
}
