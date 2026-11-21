pub type Position = Vec<u8>;
pub type ExtendedPosition = Vec<i16>; // Important to calculate steps in both positive and negative directions.

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

/// Converts a Position into a string representation for efficient lookup,
/// joining coordinates with commas (e.g., "1,2,3" for a 3D position)
pub fn into_string(pos: &Position) -> String {
    pos.iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join(",")
}
