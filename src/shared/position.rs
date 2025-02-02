pub type Position = Vec<u8>;
pub type ExtendedPosition = Vec<i16>; // Important to calculate steps in both positive and negative directions.

pub enum PositionOccupant {
    Empty,

    // The ally piece occupying the position.
    // Ally(piece)
    Ally(String),

    // The enemy piece occupying the position, alongside the team it belongs to.
    // Enemy(piece, player)
    Enemy(String, String)
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