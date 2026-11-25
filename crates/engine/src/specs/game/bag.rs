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

/// Who sees what a bag keeps, written `"visible"`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visible {
    /// Everyone: the default, as every bag was before hiding existed.
    #[default]
    #[serde(rename = "ALL")]
    All,
    /// The player the bag belongs to, and nobody else until the game is over.
    #[serde(rename = "OWNER")]
    Owner,
    /// Nobody until the game is over.
    #[serde(rename = "NONE")]
    Nobody,
}

impl Visible {
    /// Whether everyone sees it: nothing is hidden.
    pub fn is_all(&self) -> bool {
        *self == Visible::All
    }
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
    /// Whether it is a pile: it keeps its pieces in order, top first, instead of counts. A piece
    /// that enters goes on top, pieces leave from the top, and `SHUFFLE` reorders it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ordered: bool,
    /// Who sees which pieces it keeps: everyone (the default), its owner, or nobody. The game is
    /// over, and every bag is shown to everyone.
    #[serde(default, skip_serializing_if = "Visible::is_all")]
    pub visible: Visible,
    /// Whether everyone sees how many pieces a bag they cannot see into keeps (true when not
    /// written). Written only on a bag that is hidden.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shows_count: Option<bool>,
    /// Whether the rules may not read what a bag hides from the mover (see `SPEC_FORMAT.md`).
    /// Written only on a bag that is hidden.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
    /// What it holds when the game starts: piece code → count. For each owner of the bag; the
    /// pieces of a shared bag are nobody's.
    #[serde(default, skip_serializing_if = "BagStart::is_empty")]
    pub start: BagStart,
}

/// What a bag holds when the game starts: the same pieces for each owner of the bag, or, for a
/// bag each player has, what each player starts with (`{ "RAIDER": { "R": 12 }, "DEFENDER": {} }`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BagStart {
    /// Piece code → count, for each owner.
    Same(BTreeMap<String, u32>),
    /// Player → piece code → count. A player left out starts with an empty bag.
    EachPlayer(BTreeMap<String, BTreeMap<String, u32>>),
}

impl Default for BagStart {
    fn default() -> Self {
        BagStart::Same(BTreeMap::new())
    }
}

impl BagStart {
    /// Whether nobody starts with anything written (the plain form with no pieces).
    pub fn is_empty(&self) -> bool {
        matches!(self, BagStart::Same(pieces) if pieces.is_empty())
    }

    /// What `owner` starts with: the bag's pieces for the plain form, the player's own for the
    /// per-player one (a shared bag, with no owner, has none there).
    pub fn for_owner(&self, owner: Option<&str>) -> BTreeMap<String, u32> {
        match self {
            BagStart::Same(pieces) => pieces.clone(),
            BagStart::EachPlayer(each) => owner
                .and_then(|player| each.get(player))
                .cloned()
                .unwrap_or_default(),
        }
    }
}

impl BagSpec {
    /// Whether everyone sees how many pieces it keeps when they cannot see which.
    pub fn shows_count(&self) -> bool {
        self.shows_count.unwrap_or(true)
    }

    /// Whether the checker keeps the rules from reading what it hides (`strict`).
    pub fn is_strict(&self) -> bool {
        self.strict.unwrap_or(false)
    }

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
