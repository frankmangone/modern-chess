pub type Position = Vec<u8>;
pub type ExtendedPosition = Vec<i16>; // Important to calculate steps in both positive and negative directions.

pub struct PositionOccupant {
    pub piece: String,
    pub player: String
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