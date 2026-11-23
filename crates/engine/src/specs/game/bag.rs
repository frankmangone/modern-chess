//! Bags: holders off the board. A bag keeps counts of pieces, per owner and code; the hand is the
//! bag `HAND`, one per player.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Whose a bag is, written `"for"`. Without it a bag is shared.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BagFor {
    /// Each player has one.
    #[serde(rename = "EACH_PLAYER")]
    EachPlayer,
}

/// A bag, written `{ "name": "HAND", "for": "EACH_PLAYER" }`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BagSpec {
    /// Unique among the bags and the places of the board.
    pub name: String,
    /// `EACH_PLAYER` for one bag per player; shared when not written.
    #[serde(default, rename = "for", skip_serializing_if = "Option::is_none")]
    pub owners: Option<BagFor>,
    /// The most pieces it holds; unlimited when not written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<u32>,
    /// What it holds when the game starts: piece code → count. For each owner of the bag; the
    /// pieces of a shared bag are nobody's.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub start: BTreeMap<String, u32>,
}

impl BagSpec {
    /// Whether each player has one.
    pub fn for_each_player(&self) -> bool {
        self.owners.is_some()
    }
}

/// Where a bag is named: `{ "bag": "HAND", "of": "MOVER" }`. `of` is `MOVER`, `ENEMY` (the one
/// other player) or a player's name, and is written exactly for a bag that each player has.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BagRef {
    pub bag: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of: Option<String>,
}
