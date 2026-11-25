use crate::shared::IdMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(crate) mod board_changes;
pub mod move_record;
pub mod view;

use crate::logic::pieces::piece::Piece;
use crate::logic::places::holders::{self, Holder};
use crate::logic::{GamePhase, Way};
use crate::shared::{Position, Symbol};
use move_record::MoveRecord;

/// A stage of the turn being played: its name, and where the way before ended (`LAST`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StageState {
    pub name: Symbol,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::logic::play::ways::place_name::option"
    )]
    pub last: Option<Position>,
}

/// A seat an `ASK` has asked and that has not answered yet, and the stage it answers in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Asked {
    pub player: String,
    pub stage: Symbol,
}

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

    /// The stage the player to move is in, when the turn has not been handed on (a jump under
    /// way, a free turn).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<StageState>,

    /// The seats an `ASK` has asked and that have not answered yet, in the order they answer. While
    /// there are any, the player to move is the first, and `current_turn` stays with the asker.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub asked: Vec<Asked>,

    /// The stage the asker moves again in once every asked seat has answered; the turn is handed
    /// on without it. Only while `asked` is not empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask_then: Option<Symbol>,

    // Full move history, in order.
    pub history: Vec<MoveRecord>,

    // Canonical position keys recorded after each half-move, used for repetition detection.
    // Each entry is a deterministic string encoding of (active player, all pieces + their state).
    #[serde(default)]
    pub position_hashes: Vec<String>,

    /// What the holders keep: bags and the places that hold many. A holder that is empty is not
    /// here.
    #[serde(
        default,
        skip_serializing_if = "IdMap::is_empty",
        with = "position_map"
    )]
    pub holders: IdMap<Position, Holder>,

    /// The seed of the match's chance, given by whoever hosts it (see `Game::set_seed`). A game
    /// whose rules never roll or shuffle has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,

    /// How many random numbers the match has used: the next is `draw(seed, draws)`.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub draws: u64,

    /// A hand as states used to save it: player → piece_code → count. Only read: `Game::restore`
    /// moves it into the bags `HAND`.
    #[serde(default, rename = "hand", skip_serializing)]
    pub saved_hand: HashMap<String, HashMap<String, u32>>,

    /// The game's counters: player → counter name → value. A counter that is not here is 0.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub counters: HashMap<String, HashMap<String, i64>>,

    // Pieces each player has lost so far: player → piece_code → count. A piece counts as lost
    // when a move leaves fewer of its code on the board than before, and a promotion or a
    // castling rook that only changes place or form does not.
    #[serde(default)]
    pub captured: HashMap<String, HashMap<String, u32>>,
}

fn is_zero(number: &u64) -> bool {
    *number == 0
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

    use std::marker::PhantomData;

    use serde::{Deserialize, Serialize};

    use crate::shared::{IdMap, Position};

    pub fn serialize<S, V>(map: &IdMap<Position, V>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        V: Serialize,
    {
        let mut ser_map = s.serialize_map(Some(map.len()))?;
        for (pos, piece) in map {
            ser_map.serialize_entry(pos.as_str(), piece)?;
        }
        ser_map.end()
    }

    pub fn deserialize<'de, D, V>(d: D) -> Result<IdMap<Position, V>, D::Error>
    where
        D: Deserializer<'de>,
        V: Deserialize<'de>,
    {
        d.deserialize_map(PositionMapVisitor(PhantomData))
    }

    struct PositionMapVisitor<V>(PhantomData<V>);

    impl<'de, V: Deserialize<'de>> Visitor<'de> for PositionMapVisitor<V> {
        type Value = IdMap<Position, V>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a map keyed by position names")
        }

        fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
            let mut map = IdMap::default();
            while let Some((key, value)) = access.next_entry::<String, V>()? {
                map.insert(Position::from(key), value);
            }
            Ok(map)
        }
    }
}

impl GameState {
    /// The bag `HAND` of `player`, if it keeps anything.
    pub fn hand_of(&self, player: &str) -> Option<&Holder> {
        self.holders
            .get(&holders::bag_position("HAND", Some(player)))
    }

    /// How many pieces of `code` `player` has in their hand.
    pub fn hand_count(&self, player: &str, code: &str) -> u32 {
        self.hand_of(player).map_or(0, |held| held.count_of(code))
    }

    /// Puts `count` pieces of `code` in `player`'s hand: they are the player's pieces.
    pub fn put_in_hand(&mut self, player: &str, code: &str, count: u32) {
        self.holders
            .entry(holders::bag_position("HAND", Some(player)))
            .or_default()
            .add(Symbol::new(player), Symbol::new(code), count);
    }

    /// Takes one piece of `code` out of `player`'s hand; `false` when there is none.
    pub fn take_from_hand(&mut self, player: &str, code: &str) -> bool {
        let at = holders::bag_position("HAND", Some(player));
        let Some(held) = self.holders.get_mut(&at) else {
            return false;
        };
        let taken = held.take(code, 1).is_some();
        if held.is_empty() {
            self.holders.remove(&at);
        }
        taken
    }

    /// Moves a hand saved in the old shape into the bags `HAND`.
    pub fn adopt_saved_hand(&mut self) {
        for (player, counts) in std::mem::take(&mut self.saved_hand) {
            for (code, count) in counts {
                self.put_in_hand(&player, &code, count);
            }
        }
    }

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
