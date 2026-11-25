//! Tests as blocks, compiled: built once per spec with every vector already turned for each
//! player, and judged with short-circuiting.

mod build;
mod judge;

use crate::shared::PlayerMap;

use crate::logic::places::place::Place;
use crate::logic::LabelId;
use crate::shared::{ExtendedPosition, Symbol};

pub(crate) use build::{filter, Builder};
pub(crate) use judge::{
    formed_places, locate, matches, matches_held, region_places, resolve_count,
};
pub use judge::{judge, AfterTurn, TestContext};

/// A value for each player, by name.
pub type ByPlayer<T> = PlayerMap<T>;

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
        /// A capture is no threat when it leaves the attacker without a piece that passes this
        /// filter it had before.
        keeping: Option<Filter>,
        /// How many moves in a row the attacker looks ahead: 1 or 2.
        moves: u8,
    },
    InZone {
        at: Place,
        zone: String,
    },
    /// The way has so far taken off the board a piece that passes the filter.
    Removed(Filter),
    /// The position the steps so far leave, with the turn handed on, is on record at least
    /// `times` times.
    Repeats {
        times: u16,
    },
    /// The way has so far moved, along the board, a piece that passes the filter, as it stood when the turn began.
    Moved(Filter),
    /// Some place of the region passes `that`, with `EACH` at that place.
    Some {
        region: Region,
        that: Box<Test>,
    },
    /// Both places are named and are the same place.
    Same {
        a: Place,
        b: Place,
    },
    /// The player to move has no way to play (judged by end checks only).
    Stuck,
    /// The last `n` turns changed nothing.
    Passes(usize),
    /// The judging player is one of these.
    Player(Vec<String>),
    /// The judging player's counter is within `[min, max]`.
    Counter {
        name: String,
        min: i64,
        max: i64,
    },
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

/// How many pieces: written, `ALL`, or counted in the position.
#[derive(Clone, Debug)]
pub enum CountExpr {
    Number(u32),
    All,
    Of { region: Box<Region>, filter: Filter },
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
    /// The next `steps` places met by following the label from `from`, going round when it loops,
    /// without the places of the zone `skipping` (for the mover).
    Follow {
        from: Place,
        label: LabelId,
        steps: CountExpr,
        skipping: Option<String>,
    },
    /// The places of the board that pass the filter (by their piece, or, for `empty`, by
    /// holding none); every place without one.
    Board {
        filter: Option<Filter>,
    },
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
    pub codes: Vec<Symbol>,
    /// None of these codes.
    pub except: Vec<Symbol>,
    pub flag: Option<Symbol>,
    pub moved: Option<bool>,
    /// The places that hold no piece, instead of pieces: read by `COUNT` and `BOARD` only
    /// (`matches` is of pieces). The spec's checker writes it alone.
    pub empty: bool,
}
