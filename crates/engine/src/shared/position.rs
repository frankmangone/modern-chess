use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The coordinates of a grid square, as specs write them (`[4, 2]`).
pub type Coordinates = Vec<u8>;

pub type ExtendedPosition = Vec<i16>; // Important to calculate steps in both positive and negative directions.

/// A position on the board: the name of a node of the board graph. A grid square is named by
/// its coordinates joined by commas (`"4,2"`), the same text saved states use for their keys.
///
/// A name made of coordinates is written to JSON as an array (`[4, 2]`), the empty name as
/// `[]` (a drop's "from hand"), and any other name as a string, so a grid game's saved state
/// keeps its shape. Reading accepts either form.
///
/// Positions sort as their coordinates do (`"2,0"` before `"10,0"`), then names that are not
/// coordinates, by their text.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Position(String);

impl Position {
    pub fn new(name: impl Into<String>) -> Self {
        Position(name.into())
    }

    /// The name of the grid square at `coordinates`.
    pub fn from_coordinates(coordinates: &[u8]) -> Self {
        Position(
            coordinates
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(","),
        )
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The coordinates this name spells, when it is exactly a list of numbers 0 to 255 written
    /// as the engine writes them (`"4,2"`, not `"04,2"` or `"4, 2"`). The empty name is `[]`.
    pub fn coordinates(&self) -> Option<Coordinates> {
        if self.0.is_empty() {
            return Some(Vec::new());
        }
        self.0
            .split(',')
            .map(|part| {
                let value: u8 = part.parse().ok()?;
                (value.to_string() == part).then_some(value)
            })
            .collect()
    }
}

impl From<&str> for Position {
    fn from(name: &str) -> Self {
        Position(name.to_string())
    }
}

impl From<String> for Position {
    fn from(name: String) -> Self {
        Position(name)
    }
}

impl Borrow<str> for Position {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Ord for Position {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.coordinates(), other.coordinates()) {
            (Some(mine), Some(theirs)) => mine.cmp(&theirs),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => self.0.cmp(&other.0),
        }
    }
}

impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Serialize for Position {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.coordinates() {
            Some(coordinates) => coordinates.serialize(serializer),
            None => self.0.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Position {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Written {
            Coordinates(Coordinates),
            Name(String),
        }
        Ok(match Written::deserialize(deserializer)? {
            Written::Coordinates(coordinates) => Position::from_coordinates(&coordinates),
            Written::Name(name) => Position(name),
        })
    }
}

/// An N×N rotation matrix representing a player's orientation on the board, N being the
/// board's number of dimensions. Stored in row-major order: `direction[row][col]`.
///
/// Standard orientations for 2-player chess:
///   WHITE (up):    `[[1, 0], [0, 1]]`   — identity
///   BLACK (down):  `[[-1, 0], [0, -1]]` — 180° rotation
///
/// Additional orientations for 4-player chess:
///   Left  side (moves right): `[[0, 1], [-1, 0]]`  — 90° clockwise
///   Right side (moves left):  `[[0, -1], [1, 0]]`  — 90° counter-clockwise
pub type Direction = Vec<Vec<i16>>;

/// Applies a direction matrix to a step vector via 2D matrix multiplication.
/// `result[i] = Σ_j (direction[i][j] * step[j])`
pub fn apply_direction(direction: &Direction, step: &ExtendedPosition) -> ExtendedPosition {
    direction
        .iter()
        .map(|row| row.iter().zip(step.iter()).map(|(&d, &s)| d * s).sum())
        .collect()
}

/// The determinant of a square integer matrix, by fraction-free Gaussian elimination (Bareiss).
/// `None` when the matrix is not square or a value overflows; an empty matrix has determinant 1.
pub fn determinant(matrix: &Direction) -> Option<i128> {
    let size = matrix.len();
    if matrix.iter().any(|row| row.len() != size) {
        return None;
    }
    let mut rows: Vec<Vec<i128>> = matrix
        .iter()
        .map(|row| row.iter().map(|&value| value as i128).collect())
        .collect();
    let mut sign: i128 = 1;
    let mut previous_pivot: i128 = 1;

    for k in 0..size {
        let Some(pivot_row) =
            (k..size).find(|&r| rows.get(r).and_then(|row| row.get(k)) != Some(&0))
        else {
            return Some(0);
        };
        if pivot_row != k {
            rows.swap(k, pivot_row);
            sign = -sign;
        }
        let pivot_values = rows.get(k)?.clone();
        let pivot = *pivot_values.get(k)?;
        for row in rows.iter_mut().skip(k + 1) {
            let lead = *row.get(k)?;
            for j in (k + 1)..size {
                let cell = row.get_mut(j)?;
                let value = cell
                    .checked_mul(pivot)?
                    .checked_sub(lead.checked_mul(*pivot_values.get(j)?)?)?
                    .checked_div(previous_pivot)?;
                *cell = value;
            }
        }
        previous_pivot = pivot;
    }

    previous_pivot.checked_mul(sign)
}
