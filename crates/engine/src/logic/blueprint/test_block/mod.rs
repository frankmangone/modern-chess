//! Tests as blocks, compiled: built once per spec with every vector already turned for each
//! player, and judged with short-circuiting.

mod build;
mod judge;

use std::collections::HashMap;

use crate::logic::LabelId;
use crate::shared::ExtendedPosition;

pub(crate) use build::{filter, Builder};
pub use judge::{judge, TestContext};
pub(crate) use judge::{locate, matches, region_places};

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
    /// Some place of the region is attacked by `by`.
    Attacked {
        region: Region,
        by: Attacker,
    },
    InZone {
        at: Place,
        zone: String,
    },
    /// The way has so far taken off the board a piece that passes the filter.
    Removed(Filter),
    /// The position the steps so far leave, with the turn handed on, has been recorded after
    /// an earlier turn.
    Repeats,
}

/// Whose attacks an `ATTACKED` asks about, seen from the mover.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Attacker {
    /// The opponents of the mover.
    #[default]
    Enemy,
    /// The mover's own pieces.
    Mover,
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
    /// The neighbours of the places of `of` that are not in it, each once, in the order met.
    AroundRegion {
        of: Box<Region>,
        through: Through,
    },
    /// The pieces joined to the piece at `of` through neighbours of the same owner that pass
    /// `filter`, the piece at `of` included; empty when `of` holds no such piece.
    Group {
        of: Place,
        through: Through,
        filter: Filter,
    },
    /// The line through a place along each player's vector, both ways.
    Line {
        through: Place,
        along: ByPlayer<ExtendedPosition>,
    },
    /// The places of the board that pass the filter (by their piece, or, for `empty`, by
    /// holding none); every place without one.
    Board {
        filter: Option<Filter>,
    },
}

impl Region {
    /// Whether the places of the region depend on which pieces stand where (a `GROUP`, a
    /// `BOARD where`, or an `AROUND` of one), so it must be worked out on the position the
    /// steps so far have left.
    pub(crate) fn reads_pieces(&self) -> bool {
        match self {
            Region::Group { .. } | Region::Board { filter: Some(_) } => true,
            Region::AroundRegion { of, .. } => of.reads_pieces(),
            Region::At(_)
            | Region::Path { .. }
            | Region::Around { .. }
            | Region::Line { .. }
            | Region::Board { filter: None } => false,
        }
    }
}

/// The square a place is read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    Start,
    Target,
    /// The place a `FOR_EACH` is at.
    Each,
}

/// A place: an anchor, moved by each player's offset when one is written.
#[derive(Clone, Debug)]
pub struct Place {
    pub anchor: Anchor,
    pub offset: Option<ByPlayer<LabelId>>,
}

/// The neighbours an `AROUND` or a `GROUP` takes.
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
    /// The places that hold no piece, instead of pieces: read by `COUNT` and `BOARD` only
    /// (`matches` is of pieces). The spec's checker writes it alone.
    pub empty: bool,
}
