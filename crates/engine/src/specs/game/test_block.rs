//! Tests written as blocks: a tree of elementary tests over regions of the board.

use serde::{Deserialize, Deserializer, Serialize};

use crate::shared::ExtendedPosition;

/// The place of the moving piece.
pub const START: &str = "START";
/// The place where the move lands.
pub const TARGET: &str = "TARGET";
/// The place where the latest `MOVE` or `PLACE` put a piece; in a stage's `only`, where the way
/// before ended.
pub const LAST: &str = "LAST";
/// The `from` end of a `PATH`.
pub const FROM: &str = "FROM";
/// The `to` end of a `PATH`.
pub const TO: &str = "TO";
/// The place a `FOR_EACH` is at, inside its steps.
pub const EACH: &str = "EACH";
/// A filter's `owner` that takes every piece.
pub const ANY: &str = "ANY";
/// An `ATTACKED`'s `by` that takes the mover's own pieces.
pub const MOVER: &str = "MOVER";

/// A test that holds or not, written `{ "test": "COUNT", ... }`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "test", rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
pub enum TestSpec {
    /// Every test in `of` holds (an empty list holds).
    All { of: Vec<TestSpec> },
    /// At least one test in `of` holds (an empty list does not).
    Any { of: Vec<TestSpec> },
    /// `of` does not hold.
    Not { of: Box<TestSpec> },
    /// The number of pieces in a region matching a filter is within `[min, max]`. `where`
    /// defaults to every piece, `min` to 1 and `max` to no limit.
    Count {
        r#in: RegionSpec,
        #[serde(default)]
        r#where: Option<FilterSpec>,
        #[serde(default)]
        min: Option<u16>,
        #[serde(default)]
        max: Option<u16>,
    },
    /// Some place of a region is attacked: by an opponent of the mover when `by` is `ENEMY`
    /// (the default), by the mover's own pieces when it is `MOVER`.
    Attacked {
        r#in: RegionSpec,
        #[serde(default)]
        by: Option<String>,
        /// A capture counts as a threat only if the attacker still has a piece that passes this
        /// filter after it, when it had one before. Its `owner` is not written: the pieces are
        /// the attacker's.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        keeping: Option<FilterSpec>,
        /// How many moves in a row the attacker looks ahead: 1 (the default) or 2.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        moves: Option<u8>,
    },
    /// A place is in the mover's part of a zone.
    InZone { at: PlaceSpec, zone: String },
    /// The way has so far taken off the board a piece that passes `where` (`owner` seen from
    /// the mover).
    Removed { r#where: FilterSpec },
    /// The position as the steps so far leave it, with the turn handed to the next player and
    /// timed flags ticked, is on record at least `times` times (default 1: it has been recorded
    /// after an earlier turn). In an end check the position just reached is already on record.
    Repeats {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        times: Option<u16>,
    },
    /// The way has so far moved, along the board, a piece that passes `where`, as it stood
    /// when the turn began.
    Moved { r#where: FilterSpec },
    /// At least one place of the region `in` passes `that`, judged with `EACH` at that place.
    #[serde(rename = "SOME")]
    SomePlace {
        r#in: RegionSpec,
        that: Box<TestSpec>,
    },
    /// Both places are named and are the same place.
    Same { a: PlaceSpec, b: PlaceSpec },
    /// The player to move has no way to play. Judged only by an end check with `as: NEXT`.
    /// Written as a struct so that a field written on it is refused.
    Stuck {},
    /// The last `in_a_row` turns, by anyone, changed nothing.
    Passes { in_a_row: u16 },
    /// The judging player is one of `is`: the mover in a way, the `as` player in an end check.
    Player { is: Vec<String> },
    /// The judging player's counter `name` is within `[min, max]` (`min` defaults to 0, `max` to
    /// no limit).
    Counter {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<i64>,
    },
}

/// The places a test looks at, written `{ "region": "AT", ... }`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    tag = "region",
    rename_all = "SCREAMING_SNAKE_CASE",
    deny_unknown_fields
)]
pub enum RegionSpec {
    /// The one place `at`.
    At { at: PlaceSpec },
    /// The places on the straight line between `from` and `to`, with the ends `ends` names
    /// (`FROM`, `TO`). Grids only.
    Path {
        from: PlaceSpec,
        to: PlaceSpec,
        #[serde(default)]
        ends: Vec<String>,
    },
    /// The neighbours of `of` through `adjacent` (the default), `orthogonal` or a map's label.
    /// When `of` is a region: the neighbours of its places that are not in it.
    Around {
        of: AroundOf,
        #[serde(default)]
        through: Option<String>,
    },
    /// The pieces joined to the piece at `of` through neighbours (as `AROUND`'s `through`) of
    /// the same owner that pass `where`, the piece at `of` included; empty when `of` holds no
    /// such piece.
    Group {
        of: PlaceSpec,
        #[serde(default)]
        through: Option<String>,
        #[serde(default)]
        r#where: Option<FilterSpec>,
    },
    /// Every place on the line through `through` along `along`, both ways, `through` included.
    /// Grids only.
    Line {
        through: PlaceSpec,
        along: ExtendedPosition,
    },
    /// The next `steps` places met by following the map label `through` (`next` when not written)
    /// from `from`, going round when it loops; the places of the zone `skipping`, for the mover,
    /// are passed over without counting. Maps only. A label that leads to several places from
    /// one is refused when the spec is read.
    Follow {
        from: PlaceSpec,
        steps: super::action::CountSpec,
        #[serde(default)]
        through: Option<String>,
        #[serde(default)]
        skipping: Option<String>,
    },
    /// The places of the board, grid or map, that pass `where`; every place without it.
    Board {
        #[serde(default)]
        r#where: Option<FilterSpec>,
    },
}

/// What an `AROUND` is around: a place (an object with `place`) or a region (an object with
/// `region`).
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum AroundOf {
    Place(PlaceSpec),
    Region(Box<RegionSpec>),
}

impl<'de> Deserialize<'de> for AroundOf {
    /// Read by its `region` field, so that a place or a region written wrong is refused with
    /// its own message.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        let read = if value.get("region").is_some() {
            RegionSpec::deserialize(value).map(|region| AroundOf::Region(Box::new(region)))
        } else {
            PlaceSpec::deserialize(value).map(AroundOf::Place)
        };
        read.map_err(serde::de::Error::custom)
    }
}

/// A place: `START` (the moving piece's square), `TARGET` (where the move lands) or, inside a
/// `FOR_EACH`, `EACH` (the place it is at), moved by `offset`, written from the neutral player's
/// view; or a bag, written `{ "bag": "HAND", "of": "MOVER" }`. Offsets are for grids only.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceSpec {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub place: String,
    #[serde(default)]
    pub offset: Option<ExtendedPosition>,
    /// A bag instead of a place of the board; `of` says whose.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of: Option<String>,
}

/// Which pieces (or, with `empty`, which places) a test counts; every field written must hold.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FilterSpec {
    /// `ALLY`, `ENEMY` or `ANY` (the default), seen from the mover.
    #[serde(default)]
    pub owner: Option<String>,
    /// Any of these piece codes; empty means every code.
    #[serde(default)]
    pub codes: Vec<String>,
    /// None of these piece codes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub except: Vec<String>,
    /// The piece carries this state flag.
    #[serde(default)]
    pub flag: Option<String>,
    /// The piece has moved at least once (`true`) or never (`false`).
    #[serde(default)]
    pub moved: Option<bool>,
    /// The place holds no piece. Written alone, and only where a filter is of places: a
    /// `COUNT`'s and a `BOARD`'s.
    #[serde(default)]
    pub empty: bool,
}
