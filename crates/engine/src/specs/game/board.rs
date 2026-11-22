pub use super::PlayerSpec;
pub use super::TurnSpec;

//

use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashSet;

use crate::shared::Coordinates;
use crate::specs::GameSpecError;

/// The most dimensions a board may have.
pub const MAX_DIMENSIONS: usize = 8;

/// The most positions a board may have, counting disabled ones. Every position is listed when
/// moves and scores are worked out, so a board must stay small enough to list.
pub const MAX_POSITIONS: u64 = 10_000;

/// The longest a step or relative position may be along one axis: no board is wider than 255,
/// so anything longer always lands off the board.
pub const MAX_STEP: i16 = 255;

/// How a hex board's cells are arranged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HexLayout {
    /// The `size × size` grid.
    Rhombus,
    /// The `(2·size − 1) × (2·size − 1)` grid with the cells where `|q − r| ≥ size` cut away.
    Hexagon,
}

/// The shape of a board's cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoardShape {
    /// Square cells: a cell's neighbours are along each axis (`orthogonal`) or around it
    /// (`adjacent`).
    Square,
    /// Hex cells with six neighbours, `(0, ±1)`, `(±1, 0)`, `±(1, 1)`, which are both the
    /// `orthogonal` and the `adjacent` neighbourhood.
    Hex { layout: HexLayout, size: u8 },
}

/// Board spec, mostly consisting of layout specifications.
///
/// A spec writes either a square board (`dimensions`, optional `disabled_positions`) or a hex
/// board (`"shape": "hex"`, `layout`, `size`, optional `disabled_positions`). A hex board is
/// compiled to its grid when it is read, so `dimensions` and `disabled_positions` always hold
/// the grid, and `shape` says how its cells meet.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(try_from = "RawBoardSpec", into = "RawBoardSpec")]
pub struct BoardSpec {
    /// The base dimensions of the board. For instance, chess should have `vec![8u8, 8u8]`.
    pub dimensions: Vec<u8>,

    /// A set of positions that are disabled in the domain specified by the dimensions. On a
    /// hexagon, the cut corners are among them.
    pub disabled_positions: HashSet<Coordinates>,

    /// The shape of the board's cells.
    pub shape: BoardShape,
}

/// A board as a spec writes it.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct RawBoardSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    shape: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    layout: Option<HexLayout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    size: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dimensions: Option<Vec<u8>>,
    #[serde(default, deserialize_with = "deserialize_disabled_positions")]
    disabled_positions: HashSet<Coordinates>,
}

impl TryFrom<RawBoardSpec> for BoardSpec {
    type Error = String;

    fn try_from(raw: RawBoardSpec) -> Result<Self, Self::Error> {
        match raw.shape.as_deref() {
            None | Some("square") => {
                if raw.layout.is_some() || raw.size.is_some() {
                    return Err("a square board takes `dimensions`, not `layout` or `size`".into());
                }
                Ok(BoardSpec {
                    dimensions: raw.dimensions.unwrap_or_default(),
                    disabled_positions: raw.disabled_positions,
                    shape: BoardShape::Square,
                })
            }
            Some("hex") => {
                if raw.dimensions.is_some() {
                    return Err("a hex board takes `layout` and `size`, not `dimensions`".into());
                }
                let layout = raw.layout.ok_or("a hex board needs a `layout`")?;
                let size = raw
                    .size
                    .filter(|&size| size > 0)
                    .ok_or("a hex board needs a `size` of at least 1")?;
                let (side, mut disabled) = match layout {
                    HexLayout::Rhombus => (size, HashSet::new()),
                    HexLayout::Hexagon => {
                        let side = size
                            .checked_mul(2)
                            .and_then(|doubled| doubled.checked_sub(1))
                            .ok_or("the hexagon is too large")?;
                        (side, hexagon_corners(side, size))
                    }
                };
                disabled.extend(raw.disabled_positions);
                Ok(BoardSpec {
                    dimensions: vec![side, side],
                    disabled_positions: disabled,
                    shape: BoardShape::Hex { layout, size },
                })
            }
            Some(other) => Err(format!("unknown board shape `{other}`")),
        }
    }
}

impl From<BoardSpec> for RawBoardSpec {
    fn from(spec: BoardSpec) -> Self {
        match spec.shape {
            BoardShape::Square => RawBoardSpec {
                dimensions: Some(spec.dimensions),
                disabled_positions: spec.disabled_positions,
                ..RawBoardSpec::default()
            },
            BoardShape::Hex { layout, size } => {
                let side = spec.dimensions.first().copied().unwrap_or(0);
                let corners = match layout {
                    HexLayout::Rhombus => HashSet::new(),
                    HexLayout::Hexagon => hexagon_corners(side, size),
                };
                RawBoardSpec {
                    shape: Some("hex".into()),
                    layout: Some(layout),
                    size: Some(size),
                    dimensions: None,
                    disabled_positions: spec
                        .disabled_positions
                        .into_iter()
                        .filter(|cell| !corners.contains(cell))
                        .collect(),
                }
            }
        }
    }
}

/// The cells of the `side × side` grid that a hexagon of the given size cuts away: those where
/// `|q − r| ≥ size`.
fn hexagon_corners(side: u8, size: u8) -> HashSet<Coordinates> {
    let mut corners = HashSet::new();
    for q in 0..side {
        for r in 0..side {
            if q.abs_diff(r) >= size {
                corners.insert(vec![q, r]);
            }
        }
    }
    corners
}

impl BoardSpec {
    /// Refuses a board with no dimensions, a dimension of size 0, more than `MAX_DIMENSIONS`
    /// dimensions or more than `MAX_POSITIONS` positions.
    pub fn validate_size(&self) -> Result<(), GameSpecError> {
        let refused = || Err(GameSpecError::InvalidBoardSize(self.dimensions.clone()));
        if self.dimensions.is_empty() || self.dimensions.len() > MAX_DIMENSIONS {
            return refused();
        }
        let mut positions: u64 = 1;
        for &size in &self.dimensions {
            if size == 0 {
                return refused();
            }
            positions = positions.saturating_mul(u64::from(size));
        }
        if positions > MAX_POSITIONS {
            return refused();
        }
        Ok(())
    }
}

/// Custom deserialization function for `disabled_positions`.
fn deserialize_disabled_positions<'de, D>(deserializer: D) -> Result<HashSet<Coordinates>, D::Error>
where
    D: Deserializer<'de>,
{
    let vec = Option::<Vec<Coordinates>>::deserialize(deserializer)?;
    let mut map = HashSet::new();

    if let Some(positions) = vec {
        for position in positions {
            map.insert(position);
        }
    }

    Ok(map)
}

#[cfg(test)]
impl BoardSpec {
    /// Default board is just a chess board. Used for tests only.
    pub fn default() -> Self {
        BoardSpec {
            dimensions: vec![8u8, 8u8],
            disabled_positions: HashSet::new(),
            shape: BoardShape::Square,
        }
    }
}
