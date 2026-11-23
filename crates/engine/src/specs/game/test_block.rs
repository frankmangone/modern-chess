//! Tests written as blocks: a tree of elementary tests over regions of the board.

use serde::{Deserialize, Deserializer, Serialize};

use crate::shared::ExtendedPosition;

/// The place of the moving piece.
pub const START: &str = "START";
/// The place where the move lands.
pub const TARGET: &str = "TARGET";
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
    },
    /// A place is in the mover's part of a zone.
    InZone { at: PlaceSpec, zone: String },
    /// The way has so far taken off the board a piece that passes `where` (`owner` seen from
    /// the mover).
    Removed { r#where: FilterSpec },
    /// The position as the steps so far leave it, with the turn handed to the next player and
    /// timed flags ticked, has been recorded after an earlier turn. Written as a struct so that
    /// a field written on it is refused.
    Repeats {},
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
/// view. Offsets are for grids only.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceSpec {
    pub place: String,
    #[serde(default)]
    pub offset: Option<ExtendedPosition>,
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
