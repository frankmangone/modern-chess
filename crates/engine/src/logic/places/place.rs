//! A place named in a rule: an anchor, moved by each player's offset when one is written.

use crate::logic::LabelId;
use crate::shared::{PlayerMap, Position};

/// The square a place is read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    Start,
    Target,
    /// The place a `FOR_EACH` is at.
    Each,
    /// A bag: the place the `fixed` of the `Place` names for the player.
    Fixed,
}

/// A place: an anchor, moved by each player's offset when one is written.
#[derive(Clone, Debug)]
pub struct Place {
    pub anchor: Anchor,
    pub offset: Option<PlayerMap<LabelId>>,
    /// Where a bag is for each player (`Anchor::Fixed`).
    pub fixed: Option<PlayerMap<Position>>,
}
