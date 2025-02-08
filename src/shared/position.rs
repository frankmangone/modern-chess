pub type Position = Vec<u8>;
pub type ExtendedPosition = Vec<i16>; // Important to calculate steps in both positive and negative directions.

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

