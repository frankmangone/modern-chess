pub type Position = Vec<u8>;
pub type ExtendedPosition = Vec<i16>; // Important to calculate steps in both positive and negative directions.

/// A 2×2 rotation matrix representing a player's orientation on the board.
/// Stored in row-major order: `direction[row][col]`.
///
/// Standard orientations for 2-player chess:
///   WHITE (up):    `[[1, 0], [0, 1]]`   — identity
///   BLACK (down):  `[[-1, 0], [0, -1]]` — 180° rotation
///
/// Additional orientations for 4-player chess:
///   Left  side (moves right): `[[0, 1], [-1, 0]]`  — 90° clockwise
///   Right side (moves left):  `[[0, -1], [1, 0]]`  — 90° counter-clockwise
pub type Direction = [[i16; 2]; 2];

/// Applies a direction matrix to a step vector via 2D matrix multiplication.
/// `result[i] = Σ_j (direction[i][j] * step[j])`
pub fn apply_direction(direction: &Direction, step: &ExtendedPosition) -> ExtendedPosition {
    direction.iter()
        .map(|row| row.iter().zip(step.iter()).map(|(&d, &s)| d * s).sum())
        .collect()
}

/// Converts a `Position` into an `ExtendedPosition` by casting.
pub fn into_extended_position(pos: &Position) -> ExtendedPosition {
    pos.into_iter().map(|x| *x as i16).collect()
}

/// Converts an `ExtendedPosition` into a `Position` by casting.
/// This should never fail due to constraint checks during spec importing.
pub fn into_position(pos: &ExtendedPosition) -> Position {
    pos.into_iter().map(|x| *x as u8).collect()
}

/// Converts a Position into a string representation for efficient lookup,
/// joining coordinates with commas (e.g., "1,2,3" for a 3D position)
pub fn into_string(pos: &Position) -> String {
    pos.iter()
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
        .join(",")
}

