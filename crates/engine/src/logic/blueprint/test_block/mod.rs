//! Tests as blocks, compiled: built once per spec with every vector already turned for each
//! player, and judged with short-circuiting.

mod build;
mod judge;

use std::collections::HashMap;

use crate::logic::LabelId;
use crate::shared::ExtendedPosition;

pub use judge::{judge, TestContext};

/// A value for each player, by name.
pub type ByPlayer<T> = HashMap<String, T>;

/// A compiled test (see `TestSpec`).
#[derive(Clone, Debug)]
pub enum Test {
    All(Vec<Test>),
    Any(Vec<Test>),
    Not(Box<Test>),
    Count {
        region: Region,
        filter: Filter,
        min: u16,
        max: u16,
    },
    /// Some place of the region is attacked by an opponent of the mover.
    Attacked {
        region: Region,
    },
    InZone {
        at: Place,
        zone: String,
    },
}

/// The places a test looks at.
#[derive(Clone, Debug)]
pub enum Region {
    At(Place),
    /// The line from `from` to `to`, walked along each player's `unit`; `None` when the two
    /// places are written at the same spot, so no line joins them.
    Path {
        from: Place,
        to: Place,
        unit: Option<ByPlayer<LabelId>>,
        from_end: bool,
        to_end: bool,
    },
    Around {
        of: Place,
        through: Through,
    },
    /// The line through a place along each player's vector, both ways.
    Line {
        through: Place,
        along: ByPlayer<ExtendedPosition>,
    },
}

/// The square a place is read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    Start,
    Target,
}

/// A place: an anchor, moved by each player's offset when one is written.
#[derive(Clone, Debug)]
pub struct Place {
    pub anchor: Anchor,
    pub offset: Option<ByPlayer<LabelId>>,
}

/// The neighbours an `AROUND` takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Through {
    Adjacent,
    Orthogonal,
    /// The places a map's label links to.
    Linked(LabelId),
}

/// Whose pieces a filter takes, seen from the mover.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Owner {
    Ally,
    Enemy,
    #[default]
    Any,
}

/// Which pieces a test counts; the default takes every piece.
#[derive(Clone, Debug, Default)]
pub struct Filter {
    pub owner: Owner,
    /// Any of these codes; empty means every code.
    pub codes: Vec<String>,
    pub flag: Option<String>,
    pub moved: Option<bool>,
}
