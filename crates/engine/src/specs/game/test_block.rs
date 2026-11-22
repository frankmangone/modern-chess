//! Tests written as blocks: a tree of elementary tests over regions of the board.

use serde::{Deserialize, Serialize};

use crate::shared::ExtendedPosition;

/// The place of the moving piece.
pub const START: &str = "START";
/// The place where the move lands.
pub const TARGET: &str = "TARGET";
/// The `from` end of a `PATH`.
pub const FROM: &str = "FROM";
/// The `to` end of a `PATH`.
pub const TO: &str = "TO";
/// A filter's `owner` that takes every piece.
pub const ANY: &str = "ANY";

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
    /// Some place of a region is attacked by an opponent of the mover. `by` is `ENEMY`, the
    /// default and only value.
    Attacked {
        r#in: RegionSpec,
        #[serde(default)]
        by: Option<String>,
    },
    /// A place is in the mover's part of a zone.
    InZone { at: PlaceSpec, zone: String },
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
    Around {
        of: PlaceSpec,
        #[serde(default)]
        through: Option<String>,
    },
    /// Every place on the line through `through` along `along`, both ways, `through` included.
    /// Grids only.
    Line {
        through: PlaceSpec,
        along: ExtendedPosition,
    },
}

/// A place: `START` (the moving piece's square) or `TARGET` (where the move lands), moved by
/// `offset`, written from the neutral player's view. Offsets are for grids only.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceSpec {
    pub place: String,
    #[serde(default)]
    pub offset: Option<ExtendedPosition>,
}

/// Which pieces a test counts; every field written must hold.
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
}
